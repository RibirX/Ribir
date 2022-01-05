use crate::{atlas::TextureAtlas, GlRender, Primitive, RenderData, Vertex};

use lyon_tessellation::{path::Path, *};
use painter::{Brush, DeviceRect, DeviceSize, PaintCommand, Point, Rect, Size, Transform};

const TOLERANCE: f32 = 0.02;

/// `Tessellator` use to generate triangles from
pub struct Tessellator {
  // texture atlas for pure color and image to draw.
  atlas: TextureAtlas,
  render_data: RenderData,
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
}

impl Tessellator {
  /// Create a `Tessellator` with the init texture size and the maximum texture
  /// size.
  #[inline]
  pub fn new(tex_init_size: DeviceSize, tex_max_size: DeviceSize) -> Self {
    Self {
      atlas: TextureAtlas::new(tex_init_size, tex_max_size),
      render_data: RenderData::new(),
      stroke_tess: <_>::default(),
      fill_tess: <_>::default(),
    }
  }

  // todo: return render data and only generate triangles , not depends on render,
  // use callback function to instead of.
  pub fn tessellate<R: GlRender>(&mut self, cmd: Vec<PaintCommand>, render: &mut R) {
    cmd
      .into_iter()
      .for_each(|PaintCommand { path, transform, brush, path_style }| {
        let mut tolerance = TOLERANCE;
        let scale = transform.m11.max(transform.m22);
        if scale > f32::EPSILON {
          tolerance /= scale;
        }

        let _count = match path {
          painter::PaintPath::Path(path) => {
            let style_rect = self.store_style_in_atlas(&brush, render);
            let align_bounds = path_bounds_to_align_texture(&brush);
            self.add_primitive(style_rect, align_bounds, transform);
            let prim_id = self.render_data.primitives.len() as u32 - 1;
            match path_style {
              painter::PathStyle::Fill => self.fill_tess(path, tolerance, prim_id),
              painter::PathStyle::Stroke(line_width) => {
                self.stroke_tess(path, line_width, tolerance, prim_id)
              }
            }
          }
          painter::PaintPath::Text { .. } => {
            todo!("text paint as path not ready")
          }
        };
      });
    self.submit(render);
  }

  #[inline]
  pub fn atlas(&self) -> &TextureAtlas { &self.atlas }

  fn stroke_tess(&mut self, path: Path, line_width: f32, tolerance: f32, prim_id: u32) -> Count {
    let vertices = &mut self.render_data.vertices_buffer;
    self
      .stroke_tess
      .tessellate_path(
        &path,
        &StrokeOptions::tolerance(tolerance).with_line_width(line_width),
        &mut BuffersBuilder::new(vertices, move |v: StrokeVertex| Vertex {
          pixel_coords: v.position().to_array(),
          prim_id,
        }),
      )
      .unwrap()
  }

  fn fill_tess(&mut self, path: Path, tolerance: f32, prim_id: u32) -> Count {
    let vertices = &mut self.render_data.vertices_buffer;
    self
      .fill_tess
      .tessellate_path(
        &path,
        &FillOptions::tolerance(tolerance),
        &mut BuffersBuilder::new(vertices, move |v: FillVertex| Vertex {
          pixel_coords: v.position().to_array(),
          prim_id,
        }),
      )
      .unwrap()
  }

  fn add_primitive(&mut self, style_rect: DeviceRect, align_bounds: Rect, transform: Transform) {
    let primitive = Primitive {
      tex_offset: style_rect.min().to_array(),
      tex_size: style_rect.size.to_array(),
      transform: transform.to_arrays(),
      bound_min: align_bounds.min().to_array(),
      bounding_size: align_bounds.size.to_array(),
    };
    if self.render_data.primitives.last() != Some(&primitive) {
      self.render_data.primitives.push(primitive);
    }
  }

  fn store_style_in_atlas<R: GlRender>(&mut self, style: &Brush, render: &mut R) -> DeviceRect {
    match style {
      Brush::Color(c) => {
        let unit = DeviceSize::new(1, 1);
        let pos = self.atlas.store_color(c.clone()).unwrap_or_else(|_| {
          self.submit(render);
          self.atlas.clear();
          self.atlas.store_color(c.clone()).expect("never hit.")
        });

        DeviceRect::new(pos, unit)
      }
      _ => todo!("not support in early develop"),
    }
  }

  /// Consume all composed layer but not draw yet, then submit the output to
  /// render to draw.
  fn submit<R: GlRender>(&mut self, render: &mut R) {
    self.submit_to_render(render);
    self.render_data.clear();
    self.atlas.gpu_synced();
  }

  fn submit_to_render<R: GlRender>(&mut self, render: &mut R) {
    if self.render_data.has_data() {
      render.draw(&self.render_data, self.atlas.texture())
    }
  }
}

// Pure color just one pixel in texture, and always use repeat pattern, so
// zero min is ok, no matter what really bounding it is.
const COLOR_BOUNDS_TO_ALIGN_TEXTURE: Rect = Rect::new(Point::new(0., 0.), Size::new(1., 1.));

fn path_bounds_to_align_texture(style: &Brush) -> Rect {
  if let Brush::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    unimplemented!();
  }
}
