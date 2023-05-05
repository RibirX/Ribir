use std::ops::Range;

use lyon_path::geom::{euclid::num::Zero, euclid::UnknownUnit, Size};
pub type Rect<T> = lyon_path::geom::euclid::Rect<T, UnknownUnit>;
pub type Point<T> = lyon_path::geom::euclid::Point2D<T, UnknownUnit>;
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
  fn advance_glyph(&mut self, glyph: &mut Glyph<Em>, line_offset: Em, origin_text: &str) -> bool;

  fn advance(&mut self, c: Em) -> bool;

  /// cursor position relative of inline.
  fn position(&self) -> Em;
}

#[derive(Default)]
pub struct VisualLine {
  pub x: Em,
  pub y: Em,
  pub height: Em,
  pub width: Em,
  /// The glyph position is relative the line x/y
  pub glyphs: Vec<Glyph<Em>>,
}

pub struct VisualInfos {
  pub visual_lines: Vec<VisualLine>,
  /// if the typography result over the bounds provide by caller.
  pub over_bounds: bool,
  pub line_dir: PlaceLineDirection,
  pub visual_x: Em,
  pub visual_y: Em,
  pub visual_width: Em,
  pub visual_height: Em,
}

/// Typography the glyphs in a bounds.
pub struct TypographyMan<Inputs> {
  cfg: TypographyCfg,
  /// Not directly use text as inputs, but accept glyphs after text shape
  /// because both simple text and rich text can custom compose its glyph runs
  /// by text reorder result and its style .
  inputs: Inputs,
  inline_cursor: Em,
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
      inline_cursor: Em::zero(),
      visual_lines: vec![],
      over_bounds: false,
    }
  }

  pub fn typography_all(mut self) -> VisualInfos {
    while let Some(p) = self.inputs.next() {
      self.consume_paragraph(p);
      if self.is_next_line_over() {
        self.over_bounds = true;
        break;
      }
    }

    if self.cfg.line_dir.is_reverse() {
      self.visual_lines.reverse();
    }

    let (visual_width, visual_height) = self.visual_size();
    let (visual_x, visual_y) = self.adjust_lines(visual_width, visual_height);

    VisualInfos {
      visual_x,
      visual_y,
      visual_width,
      visual_height,
      visual_lines: self.visual_lines,
      over_bounds: self.over_bounds,
      line_dir: self.cfg.line_dir,
    }
  }

  fn adjust_lines(&mut self, visual_width: Em, visual_height: Em) -> (Em, Em) {
    let text_align = self.cfg.text_align.unwrap_or(TextAlign::Start);

    let bounds_width = self.cfg.bounds.width;
    let bounds_height = self.cfg.bounds.height;

    let (visual_x, visual_y) = match self.cfg.line_dir {
      PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft => {
        let mut x_offset = if self.cfg.line_dir == PlaceLineDirection::RightToLeft {
          bounds_width - visual_width
        } else {
          Em::absolute(0.)
        };
        self.visual_lines.iter_mut().for_each(move |l| {
          l.x = x_offset;
          x_offset += l.width;
        });
        (x_offset, Em::absolute(0.))
      }
      PlaceLineDirection::TopToBottom | PlaceLineDirection::BottomToTop => {
        let mut y_offset = if self.cfg.line_dir == PlaceLineDirection::BottomToTop {
          bounds_height - visual_height
        } else {
          Em::absolute(0.)
        };
        self.visual_lines.iter_mut().for_each(move |l| {
          l.y = y_offset;
          y_offset += l.height;
        });
        (Em::absolute(0.), y_offset)
      }
    };

    match (text_align, self.cfg.line_dir.is_horizontal()) {
      (TextAlign::Start, _) => {}
      (TextAlign::Center, true) => self
        .visual_lines
        .iter_mut()
        .for_each(move |l| l.y = (bounds_height - l.height) / 2.),
      (TextAlign::Center, false) => self
        .visual_lines
        .iter_mut()
        .for_each(move |l| l.x = (bounds_width - l.width) / 2.),
      (TextAlign::End, true) => self
        .visual_lines
        .iter_mut()
        .for_each(move |l| l.y = bounds_height - l.height),
      (TextAlign::End, false) => self
        .visual_lines
        .iter_mut()
        .for_each(move |l| l.x = bounds_width - l.width),
    };
    (visual_x, visual_y)
  }

  fn visual_size(&self) -> (Em, Em) {
    let mut width = Em::zero();
    let mut height = Em::zero();
    if self.cfg.line_dir.is_horizontal() {
      self.visual_lines.iter().for_each(|l| {
        width += l.width;
        height = height.max(l.height);
      })
    } else {
      self.visual_lines.iter().for_each(|l| {
        width = width.max(l.width);
        height += l.height;
      })
    };

    (width, height)
  }

  /// consume paragraph and return if early break because over boundary.
  fn consume_paragraph(&mut self, p: InputParagraph<Runs>) -> bool {
    self.visual_lines.push(<_>::default());

    if self.cfg.line_dir.is_horizontal() {
      let mut cursor = VInlineCursor { pos: self.inline_cursor };
      p.runs
        .for_each(|r| self.consume_run_with_letter_space_cursor(&r, &mut cursor));
    } else {
      let mut cursor = HInlineCursor { pos: self.inline_cursor };
      p.runs
        .for_each(|r| self.consume_run_with_letter_space_cursor(&r, &mut cursor));
    }
    self.end_line();

    false
  }

  fn consume_run_with_letter_space_cursor(
    &mut self,
    run: &Runs::Item,
    inner_cursor: &mut impl InlineCursor,
  ) {
    let letter_space = run
      .letter_space()
      .or(self.cfg.letter_space)
      .unwrap_or_else(Pixel::zero);
    if letter_space != Em::zero() {
      let mut cursor = LetterSpaceCursor::new(inner_cursor, letter_space.into());
      self.consume_run_with_bounds_cursor(run, &mut cursor);
    } else {
      self.consume_run_with_bounds_cursor(run, inner_cursor);
    }
  }

  fn consume_run_with_bounds_cursor(
    &mut self,
    run: &Runs::Item,
    inner_cursor: &mut impl InlineCursor,
  ) {
    if self.cfg.text_align != Some(TextAlign::Center) {
      let bounds = if self.cfg.line_dir.is_horizontal() {
        self.cfg.bounds.height
      } else {
        self.cfg.bounds.width
      };
      let mut cursor = BoundsCursor {
        inner_cursor,
        bounds: Em::zero()..bounds,
      };
      self.consume_run(run, &mut cursor);
    } else {
      self.consume_run(run, inner_cursor);
    }
  }

  fn consume_run(&mut self, run: &Runs::Item, cursor: &mut impl InlineCursor) {
    let font_size = run.font_size().into_em();
    let text = run.text();
    let base = run.range().start as u32;

    let line = self.visual_lines.last_mut().unwrap();
    if self.cfg.line_dir.is_horizontal() {
      line.width = line.width.max(font_size)
    } else {
      line.height = line.height.max(font_size)
    }

    // only align with the middle line in 1em, adjust after placed all runs.
    let line_offset = (font_size - Em::absolute(1.)) / 2.;
    for g in run.glyphs() {
      let mut at = g.clone();
      at.scale(font_size.value());
      let over_boundary = cursor.advance_glyph(&mut at, line_offset, text);
      at.cluster = g.cluster + base;
      self.push_glyph(at);
      if over_boundary {
        self.over_bounds = true;
        break;
      }
    }
    self.inline_cursor = cursor.position();
  }

  fn push_glyph(&mut self, g: Glyph<Em>) {
    let line = self.visual_lines.last_mut();
    line.unwrap().glyphs.push(g)
  }

  fn end_line(&mut self) {
    let line = self.visual_lines.last_mut().unwrap();
    // we will reorder the line after consumed all inputs.
    if self.cfg.line_dir.is_horizontal() {
      line.height = self.inline_cursor;
      if let Some(line_height) = self.cfg.line_height {
        line.width = line_height;
      }
    } else {
      line.width = self.inline_cursor;
      if let Some(line_height) = self.cfg.line_height {
        line.height = line_height;
      }
    }

    self.inline_cursor = Em::zero();
  }

  fn is_next_line_over(&self) -> bool {
    if self.cfg.line_dir.is_horizontal() {
      self.cfg.bounds.width < self.inline_cursor
    } else {
      self.cfg.bounds.height < self.inline_cursor
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
  fn letter_space(&self) -> Option<Pixel>;
  fn range(&self) -> Range<usize>;
}

pub struct HInlineCursor {
  pub pos: Em,
}

pub struct VInlineCursor {
  pub pos: Em,
}

pub struct LetterSpaceCursor<'a, I> {
  inner_cursor: &'a mut I,
  letter_space: Em,
}

struct BoundsCursor<'a, Inner> {
  inner_cursor: &'a mut Inner,
  bounds: Range<Em>,
}

impl<'a, I> LetterSpaceCursor<'a, I> {
  pub fn new(inner_cursor: &'a mut I, letter_space: Em) -> Self {
    Self { inner_cursor, letter_space }
  }
}

impl InlineCursor for HInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, _: &str) -> bool {
    g.x_offset += self.pos;
    g.y_offset += line_offset;
    self.pos = g.x_offset + g.x_advance;

    false
  }

  fn advance(&mut self, c: Em) -> bool {
    self.pos += c;
    false
  }

  fn position(&self) -> Em { self.pos }
}

impl InlineCursor for VInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, _: &str) -> bool {
    g.x_offset += line_offset;
    g.y_offset += self.pos;
    self.pos = g.y_offset + g.y_advance;

    false
  }

  fn advance(&mut self, c: Em) -> bool {
    self.pos += c;
    false
  }

  fn position(&self) -> Em { self.pos }
}

impl<'a, I: InlineCursor> InlineCursor for LetterSpaceCursor<'a, I> {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, origin_text: &str) -> bool {
    let cursor = &mut self.inner_cursor;
    let res = cursor.advance_glyph(g, line_offset, origin_text);

    let c = origin_text[g.cluster as usize..].chars().next().unwrap();
    if letter_spacing_char(c) {
      return cursor.advance(self.letter_space);
    }

    res
  }

  fn advance(&mut self, c: Em) -> bool { self.inner_cursor.advance(c) }

  fn position(&self) -> Em { self.inner_cursor.position() }
}

impl<'a, I: InlineCursor> InlineCursor for BoundsCursor<'a, I> {
  fn advance_glyph(&mut self, glyph: &mut Glyph<Em>, line_offset: Em, origin_text: &str) -> bool {
    self
      .inner_cursor
      .advance_glyph(glyph, line_offset, origin_text);
    !self.bounds.contains(&self.position())
  }

  fn advance(&mut self, c: Em) -> bool {
    self.inner_cursor.advance(c);
    self.bounds.contains(&self.position())
  }

  fn position(&self) -> Em { self.inner_cursor.position() }
}

impl PlaceLineDirection {
  pub fn is_horizontal(&self) -> bool {
    matches!(
      self,
      PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft
    )
  }

  pub fn is_reverse(&self) -> bool {
    matches!(
      self,
      PlaceLineDirection::RightToLeft | PlaceLineDirection::BottomToTop
    )
  }
}

impl VisualLine {
  pub fn line_height(&self, line_dir: PlaceLineDirection) -> Em {
    if line_dir.is_horizontal() {
      self.width
    } else {
      self.height
    }
  }
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
