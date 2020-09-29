use crate::{canvas::Vertex, Transform};
use lyon::{path::Path, tessellation::*};

const TOLERANCE: f32 = 0.02;
pub struct Tessellator {
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
}

impl Tessellator {
  #[inline]
  pub fn new() -> Self { <_>::default() }

  pub fn tessellate(
    &mut self,
    output: &mut VertexBuffers<Vertex, u32>,
    path: Path,
    stroke_width: Option<f32>,
    transform: &Transform,
    prim_id: u32,
  ) {
    let mut tolerance = TOLERANCE;
    let scale = transform.m11.max(transform.m22);
    if scale > f32::EPSILON {
      tolerance /= scale;
    }
    if let Some(line_width) = stroke_width {
      self
        .stroke_tess
        .tessellate_path(
          &path,
          &StrokeOptions::tolerance(tolerance).with_line_width(line_width),
          &mut BuffersBuilder::new(output, move |v: StrokeVertex| Vertex {
            pixel_coords: v.position().to_array(),
            texture_coords: [-1., -1.],
            prim_id,
          }),
        )
        .unwrap()
    } else {
      self
        .fill_tess
        .tessellate_path(
          &path,
          &FillOptions::tolerance(tolerance),
          &mut BuffersBuilder::new(output, move |v: FillVertex| Vertex {
            pixel_coords: v.position().to_array(),
            texture_coords: [-1., -1.],
            prim_id,
          }),
        )
        .unwrap()
    };
  }
}

impl Default for Tessellator {
  fn default() -> Self {
    Self {
      stroke_tess: <_>::default(),
      fill_tess: FillTessellator::new(),
    }
  }
}
