use std::ops::Range;

use ribir_geom::Size;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use crate::{Em, FontSize, Glyph, Pixel, TextAlign};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum Overflow {
  #[default]
  Clip,
  AutoWrap,
}

impl Overflow {
  fn is_auto_wrap(&self) -> bool { matches!(self, Overflow::AutoWrap) }
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
  pub text_align: TextAlign,
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
  fn advance_glyph(&mut self, glyph: &mut Glyph<Em>, line_offset: Em, origin_text: &str);

  /// advance the cursor position by a offset em
  fn advance(&mut self, offset: Em);

  /// cursor position relative of inline.
  fn position(&self) -> Em;

  fn reset(&mut self);

  fn measure(&self, glyph: &Glyph<Em>, origin_text: &str) -> Em;
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
  pub text_align: TextAlign,
  /// if the typography result over the bounds provide by caller.
  pub over_bounds: bool,
  pub line_dir: PlaceLineDirection,
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
    Self { cfg, inputs, inline_cursor: Em::ZERO, visual_lines: vec![], over_bounds: false }
  }

  pub fn typography_all(mut self) -> VisualInfos {
    while let Some(p) = self.inputs.next() {
      self.consume_paragraph(p);
    }

    if self.cfg.line_dir.is_reverse() {
      self.visual_lines.reverse();
    }

    let (visual_width, visual_height) = self.visual_size();
    self.adjust_lines(visual_width, visual_height);

    VisualInfos {
      visual_width,
      visual_height,
      text_align: self.cfg.text_align,
      visual_lines: self.visual_lines,
      over_bounds: self.over_bounds,
      line_dir: self.cfg.line_dir,
    }
  }

  fn adjust_lines(&mut self, visual_width: Em, visual_height: Em) {
    let text_align = self.cfg.text_align;

    match self.cfg.line_dir {
      PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft => {
        let mut x_offset = Em::absolute(0.);
        self.visual_lines.iter_mut().for_each(move |l| {
          l.x = x_offset;
          x_offset += l.width;
        });
      }
      PlaceLineDirection::TopToBottom | PlaceLineDirection::BottomToTop => {
        let mut y_offset = Em::absolute(0.);
        self.visual_lines.iter_mut().for_each(move |l| {
          l.y = y_offset;
          y_offset += l.height;
        });
      }
    };

    self.visual_lines.iter_mut().for_each(|l| {
      let (x, y) = text_align_offset(
        self.cfg.line_dir.is_horizontal(),
        text_align,
        visual_width,
        visual_height,
        l.width,
        l.height,
      );
      l.x += x;
      l.y += y;
    });
  }

  fn visual_size(&self) -> (Em, Em) {
    let mut width = Em::ZERO;
    let mut height = Em::ZERO;
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
    self.begin_line();

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
    &mut self, run: &Runs::Item, inner_cursor: &mut impl InlineCursor,
  ) {
    let letter_space = run
      .letter_space()
      .or(self.cfg.letter_space)
      .unwrap_or(Pixel::ZERO);
    if letter_space != Em::ZERO {
      let mut cursor = LetterSpaceCursor::new(inner_cursor, letter_space.into());
      self.consume_run(run, &mut cursor);
    } else {
      self.consume_run(run, inner_cursor);
    }
  }

  fn split_word<'a>(
    &self, run: &'a Runs::Item,
  ) -> impl Iterator<Item = impl Iterator<Item = &'a Glyph<Em>> + 'a> + 'a {
    let text = run.text();
    let mut reorder_text = String::new();

    // text and glyphs in run may in different order, so we recollect the chars.
    // and reorder_text may smaller then src text when have composited glyph,
    // like '👨‍👩‍👦‍👦'
    reorder_text.reserve(text.len());
    run.glyphs().iter().for_each(|gh| {
      reorder_text.push(
        text[gh.cluster as usize..]
          .chars()
          .next()
          .unwrap(),
      )
    });

    let mut it = reorder_text.split_word_bounds();
    let mut base = 0;
    let mut words = vec![];
    for text in it.by_ref() {
      let char_cnt = text.chars().count();
      words.push(base..char_cnt + base);
      base += char_cnt;
    }
    words.into_iter().map(move |rg| {
      rg.into_iter()
        .map(move |idx| run.glyphs().get(idx).unwrap())
    })
  }

  fn consume_run(&mut self, run: &Runs::Item, cursor: &mut impl InlineCursor) {
    let font_size = run.font_size().into_em();
    let text = run.text();
    let base = run.range().start as u32;
    let line_offset = (font_size - Em::absolute(1.)) / 2.;
    let is_auto_wrap = self.cfg.overflow.is_auto_wrap();

    let verify_line_height = |this: &mut Self| {
      let line = this.visual_lines.last_mut().unwrap();
      if this.cfg.line_dir.is_horizontal() {
        line.width = line.width.max(font_size)
      } else {
        line.height = line.height.max(font_size)
      }
    };
    let new_line = |this: &mut Self, cursor: &mut dyn InlineCursor| {
      this.end_line();
      this.begin_line();
      (verify_line_height)(this);
      cursor.reset();
    };

    let words = self
      .split_word(run)
      .map(|it| {
        it.cloned().map(|mut g| {
          g.scale(font_size.value());
          g
        })
      })
      .map(|it| {
        let word = it.collect::<Vec<_>>();
        let width = word
          .iter()
          .fold(Em::ZERO, |acc, glyph| acc + cursor.measure(glyph, text));
        (width, word)
      })
      .collect::<Vec<_>>();

    (verify_line_height)(self);
    for (width, word) in words {
      if is_auto_wrap
        && self.inline_cursor != Em::ZERO
        && self.is_over_line_bound(width + self.inline_cursor)
      {
        new_line(self, cursor);
      }

      let mut word = word.iter().peekable();

      while let Some(g) = word.peek() {
        let mut at = (*g).clone();

        cursor.advance_glyph(&mut at, line_offset, text);

        at.cluster += base;

        if self.inline_cursor == Em::ZERO
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

  fn push_glyph(&mut self, g: Glyph<Em>) {
    let line = self.visual_lines.last_mut();
    line.unwrap().glyphs.push(g)
  }

  fn begin_line(&mut self) { self.visual_lines.push(<_>::default()); }

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

    self.over_bounds |= self.is_over_line_bound(self.inline_cursor);
    self.over_bounds |= self.is_last_line_over();
    self.inline_cursor = Em::ZERO;
  }

  fn is_over_line_bound(&self, position: Em) -> bool {
    if self.cfg.text_align == TextAlign::Center {
      return false;
    }

    let eps: Em = Em(0.00001_f32);
    if self.cfg.line_dir.is_horizontal() {
      self.cfg.bounds.height + eps <= position
    } else {
      self.cfg.bounds.width + eps <= position
    }
  }

  fn is_last_line_over(&self) -> bool {
    if self.cfg.line_dir.is_horizontal() {
      self.cfg.bounds.width
        < self
          .visual_lines
          .iter()
          .fold(Em::ZERO, |acc, l| acc + l.width)
    } else {
      self.cfg.bounds.height
        < self
          .visual_lines
          .iter()
          .fold(Em::ZERO, |acc, l| acc + l.height)
    }
  }
}

pub struct InputParagraph<Runs> {
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

impl<'a, I> LetterSpaceCursor<'a, I> {
  pub fn new(inner_cursor: &'a mut I, letter_space: Em) -> Self {
    Self { inner_cursor, letter_space }
  }
}

impl InlineCursor for HInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, _: &str) {
    g.x_offset += self.pos;
    g.y_offset += line_offset;
    self.pos = g.x_offset + g.x_advance;
  }

  fn measure(&self, glyph: &Glyph<Em>, _origin_text: &str) -> Em { glyph.x_advance }

  fn advance(&mut self, c: Em) { self.pos += c; }

  fn position(&self) -> Em { self.pos }

  fn reset(&mut self) { self.pos = Em::ZERO; }
}

impl InlineCursor for VInlineCursor {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, _: &str) {
    g.x_offset += line_offset;
    g.y_offset += self.pos;
    self.pos = g.y_offset + g.y_advance;
  }

  fn advance(&mut self, c: Em) { self.pos += c; }

  fn measure(&self, glyph: &Glyph<Em>, _origin_text: &str) -> Em { glyph.y_advance }

  fn position(&self) -> Em { self.pos }

  fn reset(&mut self) { self.pos = Em::ZERO; }
}

impl<'a, I: InlineCursor> InlineCursor for LetterSpaceCursor<'a, I> {
  fn advance_glyph(&mut self, g: &mut Glyph<Em>, line_offset: Em, origin_text: &str) {
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

  fn measure(&self, glyph: &Glyph<Em>, origin_text: &str) -> Em {
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

  fn advance(&mut self, c: Em) { self.inner_cursor.advance(c) }

  fn position(&self) -> Em { self.inner_cursor.position() }

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
  pub fn line_height(&self, line_dir: PlaceLineDirection) -> Em {
    if line_dir.is_horizontal() { self.width } else { self.height }
  }
}

pub(crate) fn text_align_offset(
  is_horizontal: bool, text_align: TextAlign, bound_width: Em, bound_height: Em, visual_width: Em,
  visual_height: Em,
) -> (Em, Em) {
  match (text_align, is_horizontal) {
    (TextAlign::Start, _) => (Em::ZERO, Em::ZERO),
    (TextAlign::Center, true) => (Em::ZERO, (bound_height - visual_height) / 2.),
    (TextAlign::Center, false) => ((bound_width - visual_width) / 2., Em::ZERO),
    (TextAlign::End, true) => (Em::ZERO, bound_height - visual_height),
    (TextAlign::End, false) => (bound_width - visual_width, Em::ZERO),
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
