use std::ops::Range;

use ribir_algo::Rc;
use ribir_geom::Size;
use smallvec::{SmallVec, smallvec};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use crate::{Glyph, GlyphUnit, TextAlign, TextOverflow, shaper::ShapeResult};

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

/// Trait control how to place glyph inline.
pub trait InlineCursor {
  /// advance the cursor by a glyph, the `glyph` position is relative to self
  /// before call this method,  and relative to the cursor coordinate after
  /// call.
  fn advance_glyph(&mut self, glyph: &mut Glyph, line_offset: GlyphUnit, origin_text: &str);

  /// advance the cursor position by a offset em
  fn advance(&mut self, offset: GlyphUnit);

  /// cursor position relative of inline.
  fn position(&self) -> GlyphUnit;

  fn reset(&mut self);

  fn measure(&self, glyph: &Glyph, origin_text: &str) -> GlyphUnit;
}

#[derive(Default)]
pub struct VisualLine {
  pub x: GlyphUnit,
  pub y: GlyphUnit,
  pub height: GlyphUnit,
  pub width: GlyphUnit,
  /// The glyph position is relative the line x/y
  pub glyphs: Vec<Glyph>,
}

pub struct VisualInfos {
  pub visual_lines: SmallVec<[VisualLine; 1]>,
  pub text_align: TextAlign,
  /// if the typography result over the bounds provide by caller.
  pub over_bounds: bool,
  pub line_dir: PlaceLineDirection,
  pub visual_size: Size<GlyphUnit>,
}

/// Typography the glyphs in a bounds.
pub struct TypographyMan<Paras> {
  line_dir: PlaceLineDirection,
  text_align: TextAlign,
  line_height: GlyphUnit,
  bounds: Size<GlyphUnit>,
  overflow: TextOverflow,
  /// Not directly use text as inputs, but accept glyphs after text shape
  /// because both simple text and rich text can custom compose its glyph runs
  /// by text reorder result and its style .
  inputs: Paras,
  inline_cursor: GlyphUnit,
  visual_lines: SmallVec<[VisualLine; 1]>,
  over_bounds: bool,
}

impl<Paras> TypographyMan<Paras>
where
  Paras: DoubleEndedIterator<Item = SmallVec<[InputRun; 1]>>,
{
  pub fn new(
    inputs: Paras, line_dir: PlaceLineDirection, text_align: TextAlign, line_height: GlyphUnit,
    bounds: Size<GlyphUnit>, overflow: TextOverflow,
  ) -> Self {
    Self {
      line_dir,
      text_align,
      line_height,
      bounds,
      overflow,
      inputs,
      inline_cursor: GlyphUnit::ZERO,
      visual_lines: smallvec![],
      over_bounds: false,
    }
  }

  pub fn typography_all(mut self) -> VisualInfos {
    while let Some(p) = self.inputs.next() {
      self.consume_paragraph(p);
    }

    if self.line_dir.is_reverse() {
      self.visual_lines.reverse();
    }

    let visual_size = self.visual_size();
    self.adjust_lines(visual_size);

    VisualInfos {
      visual_size,
      text_align: self.text_align,
      visual_lines: self.visual_lines,
      over_bounds: self.over_bounds,
      line_dir: self.line_dir,
    }
  }

  fn adjust_lines(&mut self, visual_size: Size<GlyphUnit>) {
    let text_align = self.text_align;
    let lines = self.visual_lines.iter_mut();
    match self.line_dir {
      PlaceLineDirection::LeftToRight => {
        lines.fold(GlyphUnit::ZERO, move |offset, l| {
          l.x = offset;
          offset + l.width
        });
      }
      PlaceLineDirection::RightToLeft => {
        lines.fold(visual_size.width, |offset, l| {
          l.x = offset - l.width;
          l.x
        });
      }
      PlaceLineDirection::TopToBottom => {
        lines.fold(GlyphUnit::ZERO, move |offset, l| {
          l.y = offset;
          offset + l.height
        });
      }
      PlaceLineDirection::BottomToTop => {
        lines.fold(visual_size.height, |offset, l| {
          l.y = offset - l.height;
          l.y
        });
      }
    };
    if text_align != TextAlign::Start {
      self.visual_lines.iter_mut().for_each(|l| {
        if self.line_dir.is_horizontal() {
          l.y += text_align_offset(l.height, visual_size.height, text_align);
        } else {
          l.x += text_align_offset(l.width, visual_size.width, text_align);
        }
      });
    }
  }

  fn visual_size(&self) -> Size<GlyphUnit> {
    let mut width = GlyphUnit::ZERO;
    let mut height = GlyphUnit::ZERO;
    if self.line_dir.is_horizontal() {
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
    Size::new(width, height)
  }

  /// consume paragraph and return if early break because over boundary.
  fn consume_paragraph(&mut self, runs: SmallVec<[InputRun; 1]>) -> bool {
    self.begin_line();

    if self.line_dir.is_horizontal() {
      let mut cursor = VInlineCursor { pos: self.inline_cursor };
      runs
        .iter()
        .for_each(|r| self.consume_run_with_letter_space_cursor(r, &mut cursor));
    } else {
      let mut cursor = HInlineCursor { pos: self.inline_cursor };
      runs
        .iter()
        .for_each(|r| self.consume_run_with_letter_space_cursor(r, &mut cursor));
    }
    self.end_line();

    false
  }

  fn consume_run_with_letter_space_cursor(
    &mut self, run: &InputRun, inner_cursor: &mut impl InlineCursor,
  ) {
    if run.letter_space != GlyphUnit::ZERO {
      let mut cursor = LetterSpaceCursor::new(inner_cursor, run.letter_space);
      self.consume_run(run, &mut cursor);
    } else {
      self.consume_run(run, inner_cursor);
    }
  }

  fn consume_run(&mut self, run: &InputRun, cursor: &mut impl InlineCursor) {
    let font_size = run.font_size_factor * GlyphUnit::PIXELS_PER_EM as f32;
    let em = GlyphUnit::from_pixel(font_size);
    let text = run.text();
    let base = run.range.start as u32;
    let line_offset = (self.line_height - em) / 2.;
    let is_auto_wrap = self.overflow.is_auto_wrap();

    let new_line = |this: &mut Self, cursor: &mut dyn InlineCursor| {
      this.end_line();
      this.begin_line();
      cursor.reset();
    };

    for word in run.word_glyphs() {
      let width: GlyphUnit = word
        .clone()
        .fold(GlyphUnit::ZERO, |acc, g| acc + cursor.measure(&g, text));

      if is_auto_wrap
        && self.inline_cursor != GlyphUnit::ZERO
        && self.is_over_line_bound(width + self.inline_cursor)
      {
        new_line(self, cursor);
      }

      let mut word = word.peekable();
      while let Some(g) = word.peek() {
        let mut at = (*g).clone();

        cursor.advance_glyph(&mut at, line_offset, text);

        at.cluster += base;

        if self.inline_cursor == GlyphUnit::ZERO
          || !is_auto_wrap
          || !self.is_over_line_bound(cursor.position())
        {
          self.push_glyph(at);
          self.inline_cursor = cursor.position();
          word.next();
        } else {
          new_line(self, cursor);
        }
      }
    }
  }

  fn push_glyph(&mut self, g: Glyph) {
    let line = self.visual_lines.last_mut();
    line.unwrap().glyphs.push(g)
  }

  fn begin_line(&mut self) {
    let mut line = VisualLine::default();
    if self.line_dir.is_horizontal() {
      line.width = self.line_height;
    } else {
      line.height = self.line_height;
    }
    self.visual_lines.push(line);
  }

  fn end_line(&mut self) {
    let line = self.visual_lines.last_mut().unwrap();
    // we will reorder the line after consumed all inputs.
    if self.line_dir.is_horizontal() {
      line.height = self.inline_cursor;
    } else {
      line.width = self.inline_cursor;
    }
    self.over_bounds |= self.is_over_line_bound(self.inline_cursor);
    self.over_bounds |= self.is_last_line_over();
    self.inline_cursor = GlyphUnit::ZERO;
  }

  fn is_over_line_bound(&self, position: GlyphUnit) -> bool {
    if self.text_align == TextAlign::Center {
      return false;
    }

    if self.line_dir.is_horizontal() {
      self.bounds.height <= position
    } else {
      self.bounds.width <= position
    }
  }

  fn is_last_line_over(&self) -> bool {
    if self.line_dir.is_horizontal() {
      self.bounds.width
        < self
          .visual_lines
          .iter()
          .fold(GlyphUnit::ZERO, |acc, l| acc + l.width)
    } else {
      self.bounds.height
        < self
          .visual_lines
          .iter()
          .fold(GlyphUnit::ZERO, |acc, l| acc + l.height)
    }
  }
}

pub struct InputRun {
  pub(crate) shape_result: Rc<ShapeResult>,
  /// The factor relative to the standard size.
  pub(crate) font_size_factor: f32,
  pub(crate) letter_space: GlyphUnit,
  pub(crate) range: Range<usize>,
  reorder_text: String,
}

pub struct HInlineCursor {
  pub pos: GlyphUnit,
}

pub struct VInlineCursor {
  pub pos: GlyphUnit,
}

pub struct LetterSpaceCursor<'a, I> {
  inner_cursor: &'a mut I,
  letter_space: GlyphUnit,
}

impl<'a, I> LetterSpaceCursor<'a, I> {
  pub fn new(inner_cursor: &'a mut I, letter_space: GlyphUnit) -> Self {
    Self { inner_cursor, letter_space }
  }
}

impl InlineCursor for HInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph, line_offset: GlyphUnit, _: &str) {
    g.x_offset += self.pos;
    g.y_offset += line_offset;
    self.pos = g.x_offset + g.x_advance;
  }

  fn measure(&self, glyph: &Glyph, _origin_text: &str) -> GlyphUnit { glyph.x_advance }

  fn advance(&mut self, c: GlyphUnit) { self.pos += c; }

  fn position(&self) -> GlyphUnit { self.pos }

  fn reset(&mut self) { self.pos = GlyphUnit::ZERO; }
}

impl InlineCursor for VInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph, line_offset: GlyphUnit, _: &str) {
    g.x_offset += line_offset;
    g.y_offset += self.pos;
    self.pos = g.y_offset + g.y_advance;
  }

  fn advance(&mut self, c: GlyphUnit) { self.pos += c; }

  fn measure(&self, glyph: &Glyph, _origin_text: &str) -> GlyphUnit { glyph.y_advance }

  fn position(&self) -> GlyphUnit { self.pos }

  fn reset(&mut self) { self.pos = GlyphUnit::ZERO; }
}

impl<'a, I: InlineCursor> InlineCursor for LetterSpaceCursor<'a, I> {
  fn advance_glyph(&mut self, g: &mut Glyph, line_offset: GlyphUnit, origin_text: &str) {
    let cursor = &mut self.inner_cursor;
    cursor.advance_glyph(g, line_offset, origin_text);

    let c = origin_text[g.cluster as usize..]
      .chars()
      .next()
      .unwrap();
    if letter_spacing_char(c) {
      cursor.advance(self.letter_space);
    }
  }

  fn measure(&self, glyph: &Glyph, origin_text: &str) -> GlyphUnit {
    let mut advance = self.inner_cursor.measure(glyph, origin_text);

    let c = origin_text[glyph.cluster as usize..]
      .chars()
      .next()
      .unwrap();
    if letter_spacing_char(c) {
      advance += self.letter_space;
    }

    advance
  }

  fn advance(&mut self, c: GlyphUnit) { self.inner_cursor.advance(c) }

  fn position(&self) -> GlyphUnit { self.inner_cursor.position() }

  fn reset(&mut self) { self.inner_cursor.reset(); }
}

impl PlaceLineDirection {
  pub fn is_horizontal(&self) -> bool {
    matches!(self, PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft)
  }

  pub fn is_reverse(&self) -> bool {
    matches!(self, PlaceLineDirection::RightToLeft | PlaceLineDirection::BottomToTop)
  }
}

impl VisualLine {
  pub fn line_height(&self, line_dir: PlaceLineDirection) -> GlyphUnit {
    if line_dir.is_horizontal() { self.width } else { self.height }
  }

  pub fn glyphs_iter(&self, hor_line: bool) -> impl DoubleEndedIterator<Item = Glyph> + '_ {
    self.glyphs.iter().map(move |g| {
      let mut g = g.clone();
      g.x_offset += self.x;
      g.y_offset += self.y;
      if hor_line {
        g.y_advance = self.height;
      } else {
        g.x_advance = self.width;
      }
      g
    })
  }
}

pub(crate) fn text_align_offset(
  content: GlyphUnit, container: GlyphUnit, text_align: TextAlign,
) -> GlyphUnit {
  match text_align {
    TextAlign::Start => GlyphUnit::ZERO,
    TextAlign::Center => (container - content) / 2.,
    TextAlign::End => container - content,
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

impl InputRun {
  pub(crate) fn new(
    shape_result: Rc<ShapeResult>, font_size_factor: f32, letter_space: GlyphUnit,
    range: Range<usize>,
  ) -> Self {
    let text: &str = &shape_result.text;
    // text and glyphs in run may in different order, so we recollect the chars.
    // and reorder_text may smaller then src text when have composited glyph,
    // like '👨‍👩‍👦‍👦'
    let reorder_text = shape_result
      .glyphs
      .iter()
      .filter_map(|gh| text[gh.cluster as usize..].chars().next())
      .collect();
    Self { shape_result, font_size_factor, letter_space, range, reorder_text }
  }

  #[inline]
  fn text(&self) -> &str { &self.shape_result.text }

  fn word_glyphs(&self) -> impl Iterator<Item = impl Iterator<Item = Glyph> + Clone + '_> + '_ {
    let Self { reorder_text, font_size_factor, shape_result, .. } = self;
    let font_size = *font_size_factor * GlyphUnit::PIXELS_PER_EM as f32;
    reorder_text
      .split_word_bounds()
      .scan(0, move |init, w| {
        let base = *init;
        *init += w.chars().count();
        Some(
          w.chars()
            .enumerate()
            .filter_map(move |(idx, _)| shape_result.glyphs.get(base + idx))
            .map(move |g| g.clone().cast_to(font_size)),
        )
      })
  }
}
