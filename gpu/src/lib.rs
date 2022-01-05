#![feature(decl_macro, test)]
mod atlas;
pub mod error;
mod mem_texture;

#[cfg(feature = "wgpu_gl")]
pub mod wgpu_gl;

pub use mem_texture::MemTexture;
use tessellator::Tessellator;
pub mod tessellator;
use painter::{DeviceSize, PainterBackend};

use lyon_tessellation::VertexBuffers;
use zerocopy::AsBytes;

#[cfg(feature = "wgpu_gl")]
pub use wgpu_gl::wgpu_backend_headless;
#[cfg(feature = "wgpu_gl")]
pub use wgpu_gl::wgpu_backend_with_wnd;
/// A painter backend which convert `PaintCommands` to triangles and texture,
/// then submit to the gl.
pub struct GpuBackend<R: GlRender> {
  gl: R,
  tessellator: Tessellator,
}

impl<R: GlRender> PainterBackend for GpuBackend<R> {
  fn submit(&mut self, commands: Vec<painter::PaintCommand>) {
    self.tessellator.tessellate(commands, &mut self.gl)
  }

  #[inline]
  fn resize(&mut self, size: DeviceSize) { self.gl.resize(size) }
}
/// The Render that support draw the canvas result render data.
pub trait GlRender {
  fn draw(&mut self, data: &RenderData, atlas_texture: &MemTexture<4>);

  fn resize(&mut self, size: DeviceSize);
}

pub struct RenderData {
  pub vertices_buffer: VertexBuffers<Vertex, u32>,
  pub primitives: Vec<Primitive>,
}

#[repr(C)]
#[derive(AsBytes, PartialEq)]
pub struct Primitive {
  // Texture offset in texture atlas.
  pub(crate) tex_offset: [u32; 2],
  // Texture size in texture atlas.
  pub(crate) tex_size: [u32; 2],
  pub(crate) bound_min: [f32; 2],
  pub(crate) bounding_size: [f32; 2],
  pub(crate) transform: [[f32; 2]; 3],
}

/// We use a texture atlas to shader vertices, even if a pure color path.
#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Vertex {
  pub pixel_coords: [f32; 2],
  pub prim_id: u32,
}

impl RenderData {
  pub fn new() -> RenderData {
    RenderData {
      vertices_buffer: VertexBuffers::new(),
      primitives: vec![],
    }
  }

  #[inline]
  pub fn has_data(&self) -> bool {
    debug_assert_eq!(
      self.vertices_buffer.vertices.is_empty(),
      self.vertices_buffer.indices.is_empty()
    );

    !self.vertices_buffer.vertices.is_empty()
  }

  pub fn clear(&mut self) {
    self.vertices_buffer.vertices.clear();
    self.vertices_buffer.indices.clear();
    self.primitives.clear();
  }
}
