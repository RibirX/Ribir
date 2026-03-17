use ribir_geom::{Point, Rect, Vector};

use crate::{font::FontFaceId, paragraph::ClusterIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphId(pub u16);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextDrawPayload<Brush> {
  pub bounds: Rect,
  pub origin_offset: Vector,
  pub runs: Box<[DrawGlyphRun<Brush>]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawGlyphRun<Brush> {
  pub face_id: FontFaceId,
  pub logical_font_size: f32,
  pub brush: Option<Brush>,
  pub glyphs: Box<[DrawGlyph]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawGlyph {
  pub glyph_id: GlyphId,
  pub cluster: ClusterIndex,
  pub baseline_origin: Point,
  pub advance: Vector,
}
