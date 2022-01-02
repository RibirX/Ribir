#![feature(decl_macro, test, const_fn_floating_point_arithmetic)]
mod atlas;
pub mod error;
mod mem_texture;
pub mod wgpu_render;

pub use mem_texture::MemTexture;
use tessellator::Tessellator;
pub use wgpu_render::*;
mod tessellator;
use painter::DeviceSize;

const TEXTURE_INIT_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
const TEXTURE_MAX_SIZE: DeviceSize = DeviceSize::new(4096, 4096);

use lyon_tessellation::VertexBuffers;

use zerocopy::AsBytes;

/// The Render that support draw the canvas result render data.
// todo rename
pub trait CanvasRender {
  fn draw(&mut self, data: &RenderData, atlas_texture: &mut MemTexture<u32>);

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
  pub texture_coords: [f32; 2],
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

pub async fn create_canvas_with_render_from_wnd<W: raw_window_handle::HasRawWindowHandle>(
  window: &W,
  size: DeviceSize,
  tex_init_size: Option<DeviceSize>,
  tex_max_size: Option<DeviceSize>,
) -> (Tessellator, WgpuRender) {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size);
  let render = WgpuRender::wnd_render(
    window,
    size,
    tessellator.atlas().texture().size(),
    AntiAliasing::Msaa4X,
  )
  .await;

  (tessellator, render)
}

pub async fn create_canvas_with_render_headless(
  size: DeviceSize,
  tex_init_size: Option<DeviceSize>,
  tex_max_size: Option<DeviceSize>,
) -> (Tessellator, WgpuRender<surface::TextureSurface>) {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let canvas = Tessellator::new(init_size, max_size);
  let render = WgpuRender::headless_render(size, canvas.atlas().texture().size()).await;

  (canvas, render)
}
