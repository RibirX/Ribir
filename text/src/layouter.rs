use arcstr::ArcStr;

use fontdb::ID;
use lyon_path::geom::{Point, Rect, Size, Vector};
use ttf_parser::{fonts_in_collection, GlyphId};
use unic_bidi::{BidiInfo, ParagraphInfo};
use unicode_script::{Script, UnicodeScript};

use crate::{
  shaper::{Glyph, TextShaper},
  HAlign, TextDirection, VAlign,
};

#[derive(Debug, Clone)]
pub struct PixelGlyph {
  /// The font face id of the glyph.
  pub face_id: ID,
  /// How many pixels the line advances after drawing this glyph when setting
  /// text in horizontal direction.
  pub x_advance: f32,
  /// How many pixels the line advances after drawing this glyph when setting
  /// text in vertical direction.
  pub y_advance: f32,
  /// How many pixels the glyph moves on the X-axis before drawing it, this
  /// should not affect how many the line advances.
  pub x_pos: f32,
  /// How many pixels the glyph moves on the Y-axis before drawing it, this
  /// should not affect how many the line advances.
  pub y_pos: f32,
  /// The id of the glyph.
  pub glyph_id: GlyphId,
  /// An cluster of origin text as byte index.
  pub cluster: u32,
}

pub struct LayoutConfig {
  pub bounds: Rect<f32>,
  pub h_align: Option<HAlign>,
  pub v_align: Option<VAlign>,
}

pub trait Cursor {
  /// advance the cursor by a glyph, the `glyph` position is relative to self
  /// before call this method,  and relative to the cursor coordinate after
  /// call.
  /// return if the glyph is over boundary.
  fn advance_glyph(&mut self, glyph: &mut PixelGlyph, origin_text: &str) -> bool;
  fn advance_x(&mut self, x_offset: f32) -> bool;
  fn advance_y(&mut self, y_offset: f32) -> bool;
}

pub struct VisualLine {
  pos_x: f32,
  pos_y: f32,
}
pub struct Layout<Inputs, C> {
  inputs: Inputs,
  anchor_x: f32,
  anchor_y: f32,
  visual_lines: Vec<VisualLine>,
  cursor: C,
}

impl<'a, Inputs, Runs, C> Layout<Inputs, C>
where
  Inputs: DoubleEndedIterator<Item = (ParagraphInfo, Runs)>,
  Runs: Iterator<Item = InputRun<'a>>,
{
  pub fn new(inputs: Inputs) -> Self { todo!() }

  pub fn layout_para(&mut self) {
    if let Some((info, runs)) = self.inputs.next() {
      for r in runs {
        for g in r.glyphs.iter() {
          // todo
        }
      }
    }
  }
}

/// A text run with its glyphs and style
#[derive(Clone)]
pub struct InputRun<'a> {
  pub text: &'a str,
  pub glyphs: &'a [Glyph],
  pub font_size: f32,
  pub letter_space: f32,
}

impl<'a> InputRun<'a> {
  pub fn pixel_glyphs<'b, C>(&'b self, mut cursor: C) -> impl Iterator<Item = PixelGlyph> + 'b
  where
    C: Cursor + 'b,
  {
    self.glyphs.iter().map(move |g| {
      let font_size = self.font_size;
      let mut at = PixelGlyph::new(font_size, g);
      cursor.advance_glyph(&mut at, self.text);
      at
    })
  }
}

impl PixelGlyph {
  fn new(font_size: f32, g: &Glyph) -> Self {
    Self {
      face_id: g.face_id,
      x_advance: g.x_advance * font_size,
      y_advance: g.y_advance * font_size,
      x_pos: g.x_offset * font_size,
      y_pos: g.y_offset * font_size,
      glyph_id: g.glyph_id,
      cluster: g.cluster,
    }
  }
}

pub struct LinearCursor {
  x_cursor: f32,
  y_cursor: f32,
}

pub struct LetterSpaceCursor<I> {
  inner_cursor: I,
  letter_space: f32,
  is_horizontal: bool,
}

impl LinearCursor {
  #[inline]
  pub fn new(x_start: f32, y_start: f32) -> Self { Self { x_cursor: x_start, y_cursor: y_start } }
}

impl<I> LetterSpaceCursor<I> {
  #[inline]
  pub fn new(inner_cursor: I, letter_space: f32, is_horizontal: bool) -> Self {
    Self {
      inner_cursor,
      letter_space,
      is_horizontal,
    }
  }
}

impl Cursor for LinearCursor {
  #[inline]
  fn advance_glyph(&mut self, g: &mut PixelGlyph, _: &str) -> bool {
    g.x_pos += self.x_cursor;
    g.y_pos += self.y_cursor;
    self.x_cursor += g.x_advance;
    self.y_cursor += g.y_advance;

    false
  }

  #[inline]
  fn advance_x(&mut self, x_offset: f32) -> bool {
    self.x_cursor += x_offset;
    false
  }

  #[inline]
  fn advance_y(&mut self, y_offset: f32) -> bool {
    self.y_cursor += y_offset;
    false
  }
}

impl<I: Cursor> Cursor for LetterSpaceCursor<I> {
  fn advance_glyph(&mut self, g: &mut PixelGlyph, origin_text: &str) -> bool {
    let cursor = &mut self.inner_cursor;
    let res = cursor.advance_glyph(g, origin_text);

    let c = origin_text[g.cluster as usize..].chars().next().unwrap();
    if letter_spacing_char(c) {
      if self.is_horizontal {
        return cursor.advance_x(self.letter_space);
      } else {
        return cursor.advance_y(self.letter_space);
      };
    }

    res
  }

  #[inline]
  fn advance_x(&mut self, x_offset: f32) -> bool { self.inner_cursor.advance_x(x_offset) }

  #[inline]
  fn advance_y(&mut self, y_offset: f32) -> bool { self.inner_cursor.advance_y(y_offset) }
}

/*
pub struct LayoutConfig {
  pub font_size: f32,
  pub line_height: Option<f32>,
  pub letter_space: f32,
  pub h_align: Option<HAlign>,
  pub v_align: Option<VAlign>,
}

/// Layout glyphs with its glyphs and return a iterator of positioned glyph.
///
/// - *text*: the origin text of the glyphs
/// - *glyph_lines*: glyphs shaped from `text`
/// - *cfg*: config how to layout these glyphs.
/// - *bounds*: layout glyphs in the boundary, glyph out of boundary will be
///   skip and the iterator not promise glyphs order. If `bounds` not give,
///   it'll detected by how much the glyphs need.
pub fn layout_text<'a>(
  text: &'a str,
  glyph_lines: &'a [ParagraphShaped],
  cfg: &'a LayoutConfig,
  bounds: Option<Rect<f32>>,
) -> Box<dyn Iterator<Item = GlyphAt> + 'a> {
  let LayoutConfig {
    font_size, line_height, letter_space, ..
  } = cfg;

  let v_align = cfg.v_align.unwrap_or(VAlign::Top);
  match v_align {
    VAlign::Top => {
      let bounds = bounds.unwrap_or_else(|| {
        let size = glyphs_box(text, glyph_lines, *font_size, *line_height, *letter_space);
        Rect::new(Point::zero(), size)
      });
      let mut y = bounds.min_y();
      let iter = glyph_lines
        .iter()
        .map_while(move |l| {
          (y < bounds.max_y()).then(|| {
            let iter = layout_line(text, l, cfg, bounds.x_range(), y);
            y += line_height.unwrap_or(l.first_font_height) * font_size;
            iter
          })
        })
        .flatten();
      Box::new(iter)
    }
    VAlign::Center => {
      let text_size = glyphs_box(text, glyph_lines, *font_size, *line_height, *letter_space);
      let bounds = bounds.unwrap_or_else(|| Rect::new(Point::zero(), text_size));
      let mut y = bounds.min_y() + (bounds.height() - text_size.height) / 2.;
      let iter = glyph_lines
        .iter()
        .filter_map(move |l| {
          let line_pos = y;
          y += line_height.unwrap_or(l.first_font_height) * font_size;
          (bounds.min_y() < y).then(|| layout_line(text, l, cfg, bounds.x_range(), line_pos))
        })
        .flatten()
        .take_while(move |g| g.y < bounds.max_y());
      Box::new(iter)
    }
    VAlign::Bottom => {
      let bounds = bounds.unwrap_or_else(|| {
        let size = glyphs_box(text, glyph_lines, *font_size, *line_height, *letter_space);
        Rect::new(Point::zero(), size)
      });
      let mut y = bounds.max_y();
      let iter = glyph_lines
        .iter()
        .rev()
        .map_while(move |l| {
          let line_bottom = y;
          y -= line_height.unwrap_or(l.first_font_height) * font_size;
          (bounds.min_y() < line_bottom).then(|| layout_line(text, l, cfg, bounds.x_range(), y))
        })
        .flatten();
      Box::new(iter)
    }
  }
}

fn layout_line<'a>(
  text: &'a str,
  l: &'a ParagraphShaped,
  LayoutConfig { font_size, letter_space, h_align, .. }: &'a LayoutConfig,
  Range { start: min_x, end: max_x }: Range<f32>,
  y: f32,
) -> Box<dyn Iterator<Item = GlyphAt> + 'a> {
  fn run_letter_space(text: &str, run: &RunShaped, letter_space: f32) -> f32 {
    if run_has_multi_chars(text, run) {
      letter_space
    } else {
      0.
    }
  }
  let h_align = h_align.unwrap_or_else(|| {
    if l.levels[0].is_rtl() {
      HAlign::Right
    } else {
      HAlign::Left
    }
  });
  match h_align {
    HAlign::Left => {
      let mut x = 0.;
      let iter = l
        .runs
        .iter()
        .flat_map(move |run| {
          let letter_space = run_letter_space(text, run, *letter_space);
          run.glyphs.iter().map(move |g| {
            let (g_at, new_x) = step_glyph(g, x, y, *font_size, letter_space);
            x = new_x;
            g_at
          })
        })
        .take_while(move |g| g.x < max_x);
      Box::new(iter)
    }

    HAlign::Center => {
      let line_width = calc_line_width(text, l, *letter_space, *font_size);
      let mut x = min_x + ((max_x - min_x) - line_width) / 2.;

      let iter = l
        .runs
        .iter()
        .flat_map(move |run| {
          let letter_space = run_letter_space(text, run, *letter_space);
          run.glyphs.iter().filter_map(move |g| {
            let (g_at, new_x) = step_glyph(g, x, y, *font_size, letter_space);
            x = new_x;
            (new_x > min_x).then(|| g_at)
          })
        })
        .take_while(move |g| g.x < max_x);
      Box::new(iter)
    }

    HAlign::Right => {
      let mut x = max_x;
      let iter = l.runs.iter().rev().flat_map(move |run| {
        let letter_space = run_letter_space(text, run, *letter_space);
        run.glyphs.iter().rev().map_while(move |g| {
          (x > min_x).then(|| {
            x -= font_size * g.x_advance;
            let glyph_id = g.glyph_id;
            let face_id = g.face_id;
            let g_at = GlyphAt { glyph_id, face_id, x, y };
            x -= letter_space + font_size * g.x_offset;
            g_at
          })
        })
      });

      Box::new(iter)
    }
  }
}

fn run_has_multi_chars(text: &str, run: &RunShaped) -> bool {
  run
    .glyphs
    .first()
    .and_then(|g| text[g.cluster as usize..].chars().next())
    .map_or(false, letter_spacing_char)
}

fn calc_line_width(text: &str, l: &ParagraphShaped, letter_space: f32, font_size: f32) -> f32 {
  if letter_space.abs() < f32::EPSILON {
    l.width * font_size
  } else {
    l.runs
      .iter()
      .map(|r| {
        let mut w = r.width as f32 * font_size;
        if run_has_multi_chars(text, r) {
          let glyph_cnt = (r.glyphs.len() as f32 - 1.).max(0.);
          w += letter_space * glyph_cnt;
        }
        w
      })
      .sum()
  }
}

fn step_glyph(g: &Glyph, x: f32, y: f32, font_size: f32, letter_space: f32) -> (GlyphAt, f32) {
  let mut x = x + font_size * g.x_offset;
  let glyph_id = g.glyph_id;
  let face_id = g.face_id;
  let g_at = GlyphAt { glyph_id, face_id, x, y };
  x += font_size * g.x_advance + letter_space;
  (g_at, x)
}
 */

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
