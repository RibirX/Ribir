use std::{borrow::Borrow, ops::Range};

use lyon_path::geom::{euclid::num::Zero, Rect, Size};
use unicode_script::{Script, UnicodeScript};

use crate::{Em, FontSize, Glyph, Pixel, TextAlign};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overflow {
  Clip,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceLineDirection {
  /// place the line from left to right
  LeftToRight,
  /// place the line from right to let
  RightToLeft,
  /// place the line from top to bottom
  TopToBottom,
  /// place the line from bottom to top
  BottomToTop,
}
/// `mode` should match the direction use to shape text, not check if layout
/// inputs mix horizontal & vertical.
#[derive(Clone)]
pub struct TypographyCfg {
  pub line_height: Option<Em>,
  pub letter_space: Option<Pixel>,
  pub text_align: Option<TextAlign>,
  // The size glyphs can place, and hint `TypographyMan` where to early return.
  // the result of typography may over bounds.
  pub bounds: Size<Em>,
  pub line_dir: PlaceLineDirection,
  pub overflow: Overflow,
}

/// Trait control how to place glyph inline.
pub trait InlineCursor {
  /// advance the cursor by a glyph, the `glyph` position is relative to self
  /// before call this method,  and relative to the cursor coordinate after
  /// call.
  /// return if the glyph is over boundary.
  fn advance_glyph(&mut self, glyph: &mut Glyph<Em>, origin_text: &str) -> bool;

  fn advance(&mut self, c: Em) -> bool;

  /// cursor position relative of inline.
  fn position(&self) -> Em;

  fn cursor(&self) -> (Em, Em);
}

#[derive(Default)]
pub struct VisualLine {
  pub line_height: Em,
  pub glyphs: Vec<Glyph<Em>>,
}

pub struct VisualInfos {
  pub visual_lines: Vec<VisualLine>,
  pub box_rect: Rect<Em>,
  /// if the typography result over the bounds provide by caller.
  pub over_bounds: bool,
  pub line_dir: PlaceLineDirection,
}

/// pixel
pub struct TypographyMan<Inputs> {
  cfg: TypographyCfg,
  /// Not directly use text as inputs, but accept glyphs after text shape
  /// because both simple text and rich text can custom compose its glyph runs
  /// by text reorder result and its style .
  inputs: Inputs,
  x_cursor: Em,
  y_cursor: Em,
  visual_lines: Vec<VisualLine>,
  over_bounds: bool,
}

impl<Inputs, Runs> TypographyMan<Inputs>
where
  Inputs: DoubleEndedIterator<Item = InputParagraph<Runs>>,
  Runs: DoubleEndedIterator,
  Runs::Item: InputRun,
{
  pub fn new(inputs: Inputs, cfg: TypographyCfg) -> Self {
    Self {
      cfg,
      inputs,
      x_cursor: Em::zero(),
      y_cursor: Em::zero(),
      visual_lines: vec![],
      over_bounds: false,
    }
  }

  pub fn typography_all(mut self) -> VisualInfos {
    let TypographyCfg { bounds, text_align, line_dir, .. } = self.cfg;

    while let Some(p) = self.inputs.next() {
      self.consume_paragraph(p);
      if !self.is_line_over() {
        self.over_bounds = true;
        break;
      }
    }
    self.lines_reorder();
    let lines = &mut self.visual_lines;

    fn adjust_y(lines: &mut Vec<VisualLine>, f: impl Fn(&Glyph<Em>) -> Em) {
      lines.iter_mut().for_each(|l| {
        if let Some(g) = l.glyphs.last() {
          let offset = f(g);
          l.glyphs.iter_mut().for_each(|g| g.y_offset += offset);
        }
      });
    }

    fn adjust_x(lines: &mut Vec<VisualLine>, f: impl Fn(&Glyph<Em>) -> Em) {
      lines.iter_mut().for_each(|l| {
        if let Some(g) = l.glyphs.last() {
          let offset = f(g);
          l.glyphs.iter_mut().for_each(|g| g.x_offset += offset);
        }
      });
    }
    match (text_align, line_dir.is_horizontal()) {
      (Some(TextAlign::Center), true) => {
        adjust_y(lines, |g| (bounds.height - (g.y_offset + g.y_advance)) / 2.)
      }
      (Some(TextAlign::End), true) => {
        adjust_y(lines, |g| bounds.height - (g.y_offset + g.y_advance))
      }
      (Some(TextAlign::Center), false) => {
        adjust_x(lines, |g| (bounds.width - (g.x_offset + g.x_advance)) / 2.)
      }
      (Some(TextAlign::End), false) => {
        adjust_x(lines, |g| bounds.width - (g.x_offset + g.x_advance))
      }
      _ => {}
    }

    let mut x = Em::absolute(f32::MAX);
    let mut y = Em::absolute(f32::MAX);
    let mut width = Em::zero();
    let mut height = Em::zero();

    if line_dir.is_horizontal() {
      lines.iter().for_each(|l| {
        width += l.line_height;
        if let Some((f, l)) = l.glyphs.first().zip(l.glyphs.last()) {
          x = x.min(f.x_offset);
          y = y.min(f.y_offset);
          height = height.max(l.y_offset + l.y_advance - f.y_offset);
        }
      })
    } else {
      lines.iter().for_each(|l| {
        height += l.line_height;
        if let Some((f, l)) = l.glyphs.first().zip(l.glyphs.last()) {
          x = x.min(f.x_offset);
          y = y.min(f.y_offset);
          width = width.max(l.x_offset + l.x_advance - f.x_offset);
        }
      })
    };
    let box_rect = Rect::new((x, y).into(), (width, height).into());
    VisualInfos {
      box_rect,
      visual_lines: self.visual_lines,
      over_bounds: self.over_bounds,
      line_dir,
    }
  }

  /// consume paragraph and return if early break because over boundary.
  fn consume_paragraph(&mut self, p: InputParagraph<Runs>) -> bool {
    if let Some(&VisualLine { line_height, .. }) = self.visual_lines.last() {
      self.advance_to_new_line(line_height);
    }
    self.visual_lines.push(<_>::default());
    let Self { x_cursor, y_cursor, .. } = *self;
    if self.cfg.line_dir.is_horizontal() {
      p.runs.for_each(|r| {
        let cursor = VInlineCursor { x_pos: x_cursor, y_pos: y_cursor };
        self.consume_run_with_letter_space_cursor(r.borrow(), cursor)
      });
    } else {
      p.runs.for_each(|r| {
        let cursor = HInlineCursor { x_pos: x_cursor, y_pos: y_cursor };
        self.consume_run_with_letter_space_cursor(r.borrow(), cursor)
      });
    }

    if let Some(line_height) = self.cfg.line_height {
      self.visual_lines.last_mut().unwrap().line_height = line_height;
    }

    false
  }

  fn consume_run_with_letter_space_cursor(
    &mut self,
    run: &Runs::Item,
    inner_cursor: impl InlineCursor,
  ) {
    let letter_space = run
      .letter_space()
      .or(self.cfg.letter_space)
      .unwrap_or(Pixel::zero());
    if letter_space != Em::zero() {
      let cursor = LetterSpaceCursor::new(inner_cursor, letter_space.into());
      self.consume_run_with_bounds_cursor(run, cursor);
    } else {
      self.consume_run_with_bounds_cursor(run, inner_cursor);
    }
  }

  fn consume_run_with_bounds_cursor(&mut self, run: &Runs::Item, inner_cursor: impl InlineCursor) {
    if self.cfg.text_align != Some(TextAlign::Center) {
      let bounds = if self.cfg.line_dir.is_horizontal() {
        self.cfg.bounds.width
      } else {
        self.cfg.bounds.height
      };
      let cursor = BoundsCursor {
        inner_cursor,
        bounds: Em::zero()..bounds,
      };
      self.consume_run(run, cursor);
    } else {
      self.consume_run(run, inner_cursor);
    }
  }

  fn consume_run(&mut self, run: &Runs::Item, cursor: impl InlineCursor) {
    let font_size = run.font_size();
    let text = run.text();
    let glyphs = run.glyphs();

    let line = self.visual_lines.last_mut().unwrap();
    line.line_height = line.line_height.max(run.line_height());
    self.place_glyphs(cursor, font_size, text, glyphs.iter());
  }

  fn place_glyphs<'b>(
    &mut self,
    mut cursor: impl InlineCursor,
    font_size: FontSize,
    text: &str,
    runs: impl Iterator<Item = &'b Glyph<Em>>,
  ) {
    for g in runs {
      let mut at = g.clone();
      at.scale(font_size.into_em().value());
      let over_boundary = cursor.advance_glyph(&mut at, text);
      self.push_glyph(at);
      if over_boundary {
        self.over_bounds = true;
        break;
      }
    }
    let (x_cursor, y_cursor) = cursor.cursor();
    self.x_cursor = x_cursor;
    self.y_cursor = y_cursor;
  }

  fn push_glyph(&mut self, g: Glyph<Em>) {
    let line = self.visual_lines.last_mut();
    line.unwrap().glyphs.push(g)
  }

  fn advance_to_new_line(&mut self, line_height: Em) {
    // we will reorder the line after consumed all inputs.
    match self.cfg.line_dir {
      PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft => {
        self.x_cursor += line_height;
        self.y_cursor = Em::zero();
      }
      PlaceLineDirection::TopToBottom | PlaceLineDirection::BottomToTop => {
        self.y_cursor += line_height;
        self.x_cursor = Em::zero();
      }
    }

    // reset inline cursor
    if self.cfg.line_dir.is_horizontal() {
      self.y_cursor = Em::zero();
    } else {
      self.x_cursor = Em::zero();
    }
  }

  #[inline]
  fn lines_reorder(&mut self) {
    let TypographyCfg { line_dir, bounds, .. } = self.cfg;
    match line_dir {
      PlaceLineDirection::RightToLeft => {
        let o_line_cursor = Em::zero();
        let n_line_cursor = bounds.width;

        self
          .visual_lines
          .iter_mut()
          .fold((o_line_cursor, n_line_cursor), |(o, mut n), l| {
            n -= l.line_height;
            l.glyphs.iter_mut().for_each(|g| g.x_offset += n - o);
            (o + l.line_height, n)
          });
        self.visual_lines.reverse();
      }
      PlaceLineDirection::BottomToTop => {
        let o_line_cursor = Em::zero();
        let n_line_cursor = bounds.height;

        self
          .visual_lines
          .iter_mut()
          .fold((o_line_cursor, n_line_cursor), |(o, mut n), l| {
            n -= l.line_height;
            l.glyphs.iter_mut().for_each(|g| g.y_offset += n - o);
            (o + l.line_height, n)
          });
        self.visual_lines.reverse();
      }
      _ => {}
    }
  }

  fn is_line_over(&self) -> bool {
    if self.cfg.line_dir.is_horizontal() {
      self.cfg.bounds.width > self.x_cursor
    } else {
      self.cfg.bounds.height > self.y_cursor
    }
  }
}

pub struct InputParagraph<Runs> {
  pub text_align: Option<TextAlign>,
  pub runs: Runs,
}

pub trait InputRun {
  fn text(&self) -> &str;
  fn glyphs(&self) -> &[Glyph<Em>];
  fn font_size(&self) -> FontSize;
  fn line_height(&self) -> Em;
  fn letter_space(&self) -> Option<Pixel>;
}

pub struct HInlineCursor {
  pub x_pos: Em,
  pub y_pos: Em,
}

pub struct VInlineCursor {
  pub x_pos: Em,
  pub y_pos: Em,
}

pub struct LetterSpaceCursor<I> {
  inner_cursor: I,
  letter_space: Em,
}

struct BoundsCursor<Inner> {
  inner_cursor: Inner,
  bounds: Range<Em>,
}

impl<I> LetterSpaceCursor<I> {
  pub fn new(inner_cursor: I, letter_space: Em) -> Self { Self { inner_cursor, letter_space } }
}

impl InlineCursor for HInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, _: &str) -> bool {
    g.x_offset += self.x_pos;
    g.y_offset += self.y_pos;
    self.x_pos = g.x_offset + g.x_advance;

    false
  }

  fn advance(&mut self, c: Em) -> bool {
    self.x_pos += c;
    false
  }

  fn position(&self) -> Em { self.x_pos }

  fn cursor(&self) -> (Em, Em) { (self.x_pos, self.y_pos) }
}

impl InlineCursor for VInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, _: &str) -> bool {
    g.x_offset += self.x_pos;
    g.y_offset += self.y_pos;
    self.x_pos = g.y_offset + g.y_advance;

    false
  }

  fn advance(&mut self, c: Em) -> bool {
    self.y_pos += c;
    false
  }

  fn position(&self) -> Em { self.y_pos }

  fn cursor(&self) -> (Em, Em) { (self.x_pos, self.y_pos) }
}

impl<I: InlineCursor> InlineCursor for LetterSpaceCursor<I> {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, origin_text: &str) -> bool {
    let cursor = &mut self.inner_cursor;
    let res = cursor.advance_glyph(g, origin_text);

    let c = origin_text[g.cluster as usize..].chars().next().unwrap();
    if letter_spacing_char(c) {
      return cursor.advance(self.letter_space);
    }

    res
  }

  fn advance(&mut self, c: Em) -> bool { self.inner_cursor.advance(c) }

  fn position(&self) -> Em { self.inner_cursor.position() }

  fn cursor(&self) -> (Em, Em) { self.inner_cursor.cursor() }
}

impl<I: InlineCursor> InlineCursor for BoundsCursor<I> {
  fn advance_glyph(&mut self, glyph: &mut Glyph<Em>, origin_text: &str) -> bool {
    self.inner_cursor.advance_glyph(glyph, origin_text);
    !self.bounds.contains(&self.position())
  }

  fn advance(&mut self, c: Em) -> bool {
    self.inner_cursor.advance(c);
    self.bounds.contains(&self.position())
  }

  fn position(&self) -> Em { self.inner_cursor.position() }

  fn cursor(&self) -> (Em, Em) { self.inner_cursor.cursor() }
}

impl PlaceLineDirection {
  pub fn is_horizontal(&self) -> bool {
    matches!(
      self,
      PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft
    )
  }
}

impl TypographyCfg {
  pub fn is_rev_place_line(&self) -> bool { self.line_dir == PlaceLineDirection::RightToLeft }
}

/// Check if a char support apply letter spacing.
fn letter_spacing_char(c: char) -> bool {
  let script = c.script();
  // The list itself is from: https://github.com/harfbuzz/harfbuzz/issues/64
  !matches!(
    script,
    Script::Arabic
      | Script::Syriac
      | Script::Nko
      | Script::Manichaean
      | Script::Psalter_Pahlavi
      | Script::Mandaic
      | Script::Mongolian
      | Script::Phags_Pa
      | Script::Devanagari
      | Script::Bengali
      | Script::Gurmukhi
      | Script::Modi
      | Script::Sharada
      | Script::Syloti_Nagri
      | Script::Tirhuta
      | Script::Ogham
  )
}
