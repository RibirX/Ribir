use std::{
  borrow::Borrow,
  ops::Range,
  sync::{Arc, RwLock},
};

use lyon_path::geom::{euclid::num::Zero, Rect, Size};
use unicode_script::{Script, UnicodeScript};

use crate::{font_db::FontDB, Em, FontSize, Glyph, TextAlign};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overflow {
  Clip,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceLineDirection {
  /// place the line from left to right, the direction use to shape text must be
  /// vertical
  LeftToRight,
  /// place the line from right to let, the direction use to shape text must be
  /// vertical
  RightToLeft,
  /// place the line from top to bottom, the direction use to shape text must be
  /// vertical
  TopToBottom,
}
/// `mode` should match the direction use to shape text, not check if layout
/// inputs mix horizontal & vertical.
#[derive(Clone)]
pub struct TypographyCfg {
  pub line_height: Option<Em>,
  pub letter_space: Option<Em>,
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
  // todo: unset
  pub line_height: Em,
  pub glyphs: Vec<Glyph<Em>>,
}

#[derive(Default)]
pub struct VisualInfos {
  pub visual_lines: Vec<VisualLine>,
  pub box_rect: Rect<Em>,
  /// if the typography result over the bounds provide by caller.
  pub over_bounds: bool,
}

/// pixel
pub struct TypographyMan<Inputs> {
  font_db: Arc<RwLock<FontDB>>,
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
  pub fn new(inputs: Inputs, cfg: TypographyCfg, font_db: Arc<RwLock<FontDB>>) -> Self {
    Self {
      font_db,
      cfg,
      inputs,
      x_cursor: Em::zero(),
      y_cursor: Em::zero(),
      visual_lines: <_>::default(),
      over_bounds: false,
    }
  }

  pub fn typography_all(mut self) -> VisualInfos {
    let TypographyCfg { bounds, text_align, line_dir, .. } = self.cfg;
    while let Some(p) = self.next_input_paragraph() {
      self.consume_paragraph(p);
      if !bounds.contains((self.x_cursor, self.y_cursor).into()) {
        self.over_bounds = true;
        break;
      }
    }
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
      (Some(TextAlign::Center), false) => {
        adjust_y(lines, |g| (bounds.height - (g.y_offset + g.y_advance)) / 2.)
      }
      (Some(TextAlign::End), false) => {
        adjust_y(lines, |g| bounds.height - (g.y_offset + g.y_advance))
      }
      (Some(TextAlign::Center), true) => {
        adjust_x(lines, |g| (bounds.width - (g.x_offset + g.x_advance)) / 2.)
      }
      (Some(TextAlign::End), true) => {
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
    }
  }

  pub fn next_input_paragraph(&mut self) -> Option<InputParagraph<Runs>> {
    if self.cfg.is_rev_place_line() {
      self.inputs.next_back()
    } else {
      self.inputs.next()
    }
  }

  /// consume paragraph and return if early break because over boundary.
  fn consume_paragraph(&mut self, p: InputParagraph<Runs>) -> bool {
    let mut runs = p.runs.peekable();
    if !self.visual_lines.is_empty() || self.cfg.is_rev_place_line() {
      if let Some(r) = runs.peek() {
        let input_run = r.borrow();
        let line_height = self.line_height_with_glyph(input_run.glyphs().first());
        self.advance_to_new_line(line_height * input_run.font_size().into_em());
      }
    }

    let Self { x_cursor, y_cursor, .. } = *self;
    if self.cfg.line_dir.is_horizontal() {
      runs.for_each(|r| {
        let cursor = HInlineCursor { x_pos: x_cursor, y_pos: y_cursor };
        self.consume_run_with_letter_space_cursor(r.borrow(), cursor)
      });
    } else {
      runs.for_each(|r| {
        let cursor = VInlineCursor { x_pos: x_cursor, y_pos: y_cursor };
        self.consume_run_with_letter_space_cursor(r.borrow(), cursor)
      });
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
      .unwrap_or(Em::zero());
    if letter_space != Em::zero() {
      let cursor = LetterSpaceCursor::new(inner_cursor, letter_space);
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

  fn line_height_with_glyph(&self, g: Option<&Glyph<Em>>) -> Em {
    self
      .cfg
      .line_height
      .or_else(|| {
        g.and_then(|g| {
          let face = self.font_db.read().unwrap();
          face.try_get_face_data(g.face_id).map(|face| {
            let p_gap = match self.cfg.line_dir {
              PlaceLineDirection::LeftToRight | PlaceLineDirection::RightToLeft => {
                face.vertical_line_gap().unwrap_or_else(|| face.line_gap())
              }
              PlaceLineDirection::TopToBottom => face.line_gap(),
            };
            Em::absolute(p_gap as f32 / face.units_per_em() as f32)
          })
        })
      })
      .unwrap_or(Em::absolute(1.))
  }

  fn advance_to_new_line(&mut self, c: Em) {
    match self.cfg.line_dir {
      PlaceLineDirection::LeftToRight => self.x_cursor += c,
      PlaceLineDirection::RightToLeft => self.x_cursor -= c,
      PlaceLineDirection::TopToBottom => self.y_cursor += c,
    }

    // reset inline cursor
    if self.cfg.line_dir.is_horizontal() {
      self.y_cursor = Em::zero();
    } else {
      if let Some(TextAlign::End) = self.cfg.text_align {
        self.x_cursor = self.cfg.bounds.width;
      } else {
        self.x_cursor = Em::zero();
      }
    }

    self.visual_lines.push(VisualLine::default())
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
  fn letter_space(&self) -> Option<Em>;
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
    self.bounds.contains(&self.position())
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

// #[cfg(test)]
// mod tests {
//   use super::*;
//   use crate::{shaper::*, FontFace, FontFamily};

//   #[test]
//   fn simple_text_bounds() {
//     let shaper = TextShaper::default();
//     let path = env!("CARGO_MANIFEST_DIR").to_owned() +
// "/../fonts/DejaVuSans.ttf";     let _ =
// shaper.font_db_mut().load_font_file(path);

//     let ids = shaper.font_db().select_all_match(&FontFace {
//       families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
//       ..<_>::default()
//     });

//     let text = "Hello

//     world!";
//     let glyphs = shaper.shape_text(text, &ids);
//     let size = glyphs_box(text, glyphs.as_ref(), 14., None, 1.);
//     assert_eq!(size, Size::new(70.96094, 81.484375));
//   }

//   #[test]
//   fn simple_layout_text() {
//     let shaper = TextShaper::default();
//     let path = env!("CARGO_MANIFEST_DIR").to_owned() +
// "/../fonts/DejaVuSans.ttf";     let _ =
// shaper.font_db_mut().load_font_file(path);

//     let ids = shaper.font_db().select_all_match(&FontFace {
//       families: Box::new([FontFamily::Name("DejaVu Sans".into())]),
//       ..<_>::default()
//     });
//     let text = "Hello--------\nworld!";
//     let glyphs = shaper.shape_text(text, &ids);
//     let mut cfg = LayoutConfig {
//       font_size: 10.,
//       letter_space: 2.,
//       h_align: None,
//       v_align: None,
//       line_height: None,
//     };

//     let layout = |cfg: &LayoutConfig, bounds: Option<Rect<f32>>| {
//       layout_text(text, &glyphs, cfg, bounds)
//         .map(|g| (g.x, g.y))
//         .collect::<Vec<_>>()
//     };

//     let not_bounds = layout(&cfg, None);
//     assert_eq!(
//       &not_bounds,
//       &[
//         (0.0, 0.0),
//         (9.519531, 0.0),
//         (17.671875, 0.0),
//         (22.450195, 0.0),
//         (27.228516, 0.0),
//         (35.532227, 0.0),
//         (41.140625, 0.0),
//         (46.749023, 0.0),
//         (52.35742, 0.0),
//         (57.96582, 0.0),
//         (63.57422, 0.0),
//         (69.18262, 0.0),
//         (74.791016, 0.0),
//         (80.399414, 0.0),
//         // second line
//         (0.0, 11.640625),
//         (10.178711, 11.640625),
//         (18.296875, 11.640625),
//         (24.408203, 11.640625),
//         (29.186523, 11.640625),
//         (37.53418, 11.640625)
//       ]
//     );

//     cfg.h_align = Some(HAlign::Right);
//     let r_align = layout(&cfg, None);
//     assert_eq!(
//       &r_align,
//       &[
//         (80.399414, 0.0),
//         (74.791016, 0.0),
//         (69.18262, 0.0),
//         (63.57422, 0.0),
//         (57.96582, 0.0),
//         (52.35742, 0.0),
//         (46.749023, 0.0),
//         (41.140625, 0.0),
//         (35.532227, 0.0),
//         (27.228516, 0.0),
//         (22.450195, 0.0),
//         (17.671875, 0.0),
//         (9.519531, 0.0),
//         (0.0, 0.0),
//         // second line.
//         (82.3916, 11.640625),
//         (74.043945, 11.640625),
//         (69.265625, 11.640625),
//         (63.154297, 11.640625),
//         (55.036133, 11.640625),
//         (44.85742, 11.640625)
//       ]
//     );

//     cfg.h_align = None;
//     cfg.v_align = Some(VAlign::Bottom);

//     let bottom = layout(&cfg, None);
//     assert_eq!(
//       &bottom,
//       &[
//         // second line
//         (0.0, 11.640625),
//         (10.178711, 11.640625),
//         (18.296875, 11.640625),
//         (24.408203, 11.640625),
//         (29.186523, 11.640625),
//         (37.53418, 11.640625),
//         (0.0, 0.0),
//         // first line
//         (9.519531, 0.0),
//         (17.671875, 0.0),
//         (22.450195, 0.0),
//         (27.228516, 0.0),
//         (35.532227, 0.0),
//         (41.140625, 0.0),
//         (46.749023, 0.0),
//         (52.35742, 0.0),
//         (57.96582, 0.0),
//         (63.57422, 0.0),
//         (69.18262, 0.0),
//         (74.791016, 0.0),
//         (80.399414, 0.0)
//       ]
//     );

//     cfg.h_align = Some(HAlign::Center);
//     cfg.v_align = Some(VAlign::Center);
//     let center_clip = layout(&cfg, Some(Rect::from_size(Size::new(40.,
// 15.))));     assert_eq!(
//       &center_clip,
//       &[
//         // first line
//         (-0.75, -4.140625)  if let Some(letter_space) = letter_space {
//         (17.52539, 7.5),
//         (23.636719, 7.5),
//         (28.41504, 7.5),
//         (36.762695, 7.5)
//       ]
//     );
//   }
// }
