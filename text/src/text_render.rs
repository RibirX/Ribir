use crate::{font_db::FontDB, Em, FontFace, FontSize, GlyphBound, Pixel};
use ribir_algo::ShareResource;
use ribir_geom::{Transform, Vector};
use ribir_painter::{Brush, Painter, Path, PathPaintStyle};
use std::sync::{Arc, RwLock};

/// Encapsulates the text style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
  /// The size of glyphs (in logical pixels) to use when painting the text.
  pub font_size: FontSize,
  /// The font face to use when painting the text.
  // todo: use ids instead of
  pub font_face: FontFace,
  /// Not support now.
  pub letter_space: Option<Pixel>,
  /// The factor use to multiplied by the font size to specify the text line
  /// height.
  pub line_height: Option<Em>,
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_size: FontSize::Pixel(14.0.into()),
      font_face: Default::default(),
      letter_space: None,
      line_height: None,
    }
  }
}

pub fn paint_glyphs(
  painter: &mut Painter,
  font_db: Arc<RwLock<FontDB>>,
  glyphs: impl Iterator<Item = GlyphBound>,
  brush: Brush,
  font_size: f32,
  path_style: &PathPaintStyle,
) {
  glyphs.for_each(|g| {
    let mut font_db = font_db.write().unwrap();
    let face = font_db.try_get_face_data(g.face_id);

    if let Some(face) = face {
      let unit = face.units_per_em() as f32;
      let scale = font_size / unit;
      if let Some(path) = face.outline_glyph(g.glyph_id) {
        let ts = Transform::scale(1., -1.)
          .then_translate((0., unit).into())
          .then_scale(scale, scale)
          .then_translate(Vector::new(g.bound.min_x(), g.bound.min_y()));
        let path = Path::from(path).transform(&ts);
        painter.set_brush(brush.clone());
        match path_style {
          PathPaintStyle::Fill => {
            painter.fill_path(path);
          }
          PathPaintStyle::Stroke(stroke) => {
            painter.set_strokes(stroke.clone()).stroke_path(path);
          }
        }
      }
    }
  });
}
