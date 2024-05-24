use std::{cell::RefCell, rc::Rc};

use ribir_geom::{Rect, Size};
use ribir_painter::{Brush, Painter, Path, PathStyle};

use crate::{font_db::FontDB, Em, FontFace, FontSize, GlyphBound, Pixel, VisualGlyphs};

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

/// draw the text glyphs within the box_rect, with the given brush font_size and
/// path style
pub fn draw_glyphs_in_rect(
  painter: &mut Painter, visual_glyphs: VisualGlyphs, box_rect: Rect, brush: Brush, font_size: f32,
  path_style: &PathStyle, font_db: Rc<RefCell<FontDB>>,
) {
  let visual_rect = visual_glyphs.visual_rect();
  let Some(paint_rect) = painter.intersection_paint_bounds(&box_rect) else {
    return;
  };
  if !paint_rect.contains_rect(&visual_rect) {
    painter.clip(Path::rect(&paint_rect));
  }
  painter.translate(visual_rect.origin.x, visual_rect.origin.y);
  draw_glyphs(
    painter,
    visual_glyphs.glyph_bounds_in_rect(&paint_rect),
    brush,
    font_size,
    path_style,
    font_db,
  );
}

/// draw the glyphs with the given brush, font_size and path style
pub fn draw_glyphs(
  painter: &mut Painter, glyphs: impl Iterator<Item = GlyphBound>, brush: Brush, font_size: f32,
  path_style: &PathStyle, font_db: Rc<RefCell<FontDB>>,
) {
  glyphs.for_each(|g| {
    let font_db = font_db.borrow();
    let face = font_db.try_get_face_data(g.face_id);

    if let Some(face) = face {
      let unit = face.units_per_em() as f32;
      let scale = font_size / unit;
      if let Some(path) = face.outline_glyph(g.glyph_id, path_style) {
        let mut painter = painter.save_guard();
        painter
          .translate(g.bound.min_x(), g.bound.min_y())
          .scale(scale, -scale)
          .translate(0., -unit);

        painter.set_brush(brush.clone()).fill_path(path);
      } else if let Some(svg) = face.glyph_svg_image(g.glyph_id) {
        let mut painter = painter.save_guard();

        let grid_scale = face
          .vertical_height()
          .map(|h| h as f32 / face.units_per_em() as f32)
          .unwrap_or(1.)
          .max(1.);
        let size = svg.size;
        let bound_size = g.bound.size;
        let scale =
          (bound_size.width / size.width).min(bound_size.height / size.height) / grid_scale;
        painter
          .translate(g.bound.min_x(), g.bound.min_y())
          .scale(scale, scale)
          .draw_svg(&svg);
      } else if let Some(img) = face.glyph_raster_image(g.glyph_id, (unit / font_size) as u16) {
        let m_width = img.width() as f32;
        let m_height = img.height() as f32;
        let scale = (g.bound.width() / m_width).min(g.bound.height() / m_height);

        let x_offset = g.bound.min_x() + (g.bound.width() - (m_width * scale)) / 2.;
        let y_offset = g.bound.min_y() + (g.bound.height() - (m_height * scale)) / 2.;
        let mut painter = painter.save_guard();
        painter
          .translate(x_offset, y_offset)
          .scale(scale, scale)
          .draw_img(img, &Rect::from_size(Size::new(m_width, m_height)), &None);
      }
    }
  });
}
