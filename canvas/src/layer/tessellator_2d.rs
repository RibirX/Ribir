use crate::{canvas::Vertex, Transform};
pub use lyon::{
  path::{builder::PathBuilder, traits::PathIterator, Path, Winding},
  tessellation::*,
};

const TOLERANCE: f32 = 0.5;
pub struct Tessellator {
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
}

impl Tessellator {
  pub fn new() -> Self {
    Self {
      stroke_tess: <_>::default(),
      fill_tess: FillTessellator::new(),
    }
  }

  pub fn tessellate(
    &mut self,
    output: &mut VertexBuffers<Vertex, u32>,
    path: Path,
    stroke_width: Option<f32>,
    _transform: &Transform,
    prim_id: u32,
  ) {
    // todo: use transform to generate TOLERANCE;
    if let Some(line_width) = stroke_width {
      self
        .stroke_tess
        .tessellate_path(
          &path,
          &StrokeOptions::tolerance(TOLERANCE).with_line_width(line_width),
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
          &FillOptions::tolerance(TOLERANCE),
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
