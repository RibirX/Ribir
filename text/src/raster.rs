use ribir_geom::Point;

use crate::{FontFaceId, FontFaceMetrics, paint::GlyphId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RasterBitmapFormat {
  Rgba8,
  Alpha8,
}

impl RasterBitmapFormat {
  pub fn is_only_alpha(self) -> bool { matches!(self, Self::Alpha8) }
}

#[derive(Debug, Clone)]
pub struct RasterBitmap {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32,
  pub format: RasterBitmapFormat,
  pub placement: Point,
}

pub trait GlyphRasterSource {
  fn face_metrics(&self, face_id: FontFaceId) -> Option<FontFaceMetrics>;

  fn raster_bitmap(
    &self, face_id: FontFaceId, glyph_id: GlyphId, font_ppem: u16,
  ) -> Option<RasterBitmap>;

  fn raster_svg(&self, face_id: FontFaceId, glyph_id: GlyphId) -> Option<String>;
}
