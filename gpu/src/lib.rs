pub mod error;
use ribir_painter::{
  image::ColorFormat, AntiAliasing, DeviceRect, DeviceSize, Texture, VertexBuffers,
};
use std::ops::Range;
mod gpu_backend;
use zerocopy::AsBytes;

#[cfg(feature = "wgpu")]
pub mod wgpu_impl;
#[cfg(feature = "wgpu")]
pub use wgpu_impl::*;

pub use gpu_backend::GPUBackend;

#[derive(Clone)]
pub enum DrawIndices {
  Color(Range<u32>),
  Texture(Range<u32>),
  Gradient(Range<u32>),
}

/// Trait to help implement a gpu backend.
///
/// The call graph:
///
/// -- begin_frame()
///  +-->--start_draw_phase() -->-----------------------+   
///  | +->- new_texture()----+                          |
///  | +-<-------<------<----+                          |    
///  | |                                                |
///  | v                                                |
///  | +--> load_textures()                             v
///  |   -> load_color_primitives()                     |
///  |   -> load_texture_primitives()                   |
///  |   -> load_triangles_buffer()                     |
///  |                                                  |
///  |   -> load_alpha_path_buffer()                    |
///  |   -> + draw_alpha_triangles_with_scissor()--+    |
///  |      +----<-----------<---------------------+    |
///  |                                                  |
///  |   -> + draw_alpha_triangles()---------------+    |
///  |      +----<-----------<---------------------+    |
///  |                                                  |
///  |   -> +--- draw_triangles()----+                  |
///  ^      +-------<----------------+                  |
///  |                                                  |
///  +----- end_draw_phase() ---<-----------------------+
///  |
///  V
/// -+ ->- end_frame()
pub trait GPUBackendImpl {
  type Texture: Texture;

  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing);

  /// A frame start, call once per frame
  fn begin_frame(&mut self);

  /// A draw phase start. A frame may have many draw phase.
  fn start_draw_phase(&mut self);

  /// Create a texture.
  fn new_texture(&mut self, size: DeviceSize, format: ColorFormat) -> Self::Texture;

  /// load textures that will be use in this draw phase
  fn load_textures<'a, Iter>(&mut self, textures: Iter)
  where
    Iter: Iterator<Item = &'a Self::Texture> + 'a,
    Self::Texture: 'a;

  /// load color primitives that the current draw phase will use, called at
  /// most once per draw phase.
  fn load_color_primitives(&mut self, primitives: &[ColorPrimitive]);

  /// load texture primitives that the current draw phase will use, called at
  /// most once per draw phase.
  fn load_texture_primitives(&mut self, primitives: &[TexturePrimitive]);

  /// Load the vertices and indices buffer that `draw_alpha_triangles` &
  /// `draw_alpha_triangles_with_scissor` will use.
  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<()>);

  /// Draw triangles only alpha channel with 1.0. Caller guarantee the texture
  /// format is `ColorFormat::Alpha8`, caller will try to batch as much as
  /// possible, but also possibly call multi times in a frame.
  fn draw_alpha_triangles(&mut self, indices: &Range<u32>, texture: &mut Self::Texture);

  /// Same behavior as `draw_alpha_triangles`, but the Vertex with a offset and
  /// gives a clip rectangle for the texture, the path should only painting in
  /// the rectangle.
  fn draw_alpha_triangles_with_scissor(
    &mut self,
    indices: &Range<u32>,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
  );

  /// Load the vertices and indices buffer that `draw_triangles` will use.
  fn load_triangles_vertices(&mut self, buffers: &VertexBuffers<u32>);

  /// A batch draws for a texture.
  fn draw_triangles(
    &mut self,
    texture: &mut Self::Texture,
    scissor: DeviceRect,
    commands: &[DrawIndices],
  );

  /// A draw phase end
  fn end_draw_phase(&mut self);

  /// A frame end, call once per frame
  fn end_frame(&mut self);
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct ColorPrimitive {
  /// brush's Rgba color
  pub color: [f32; 4],
  /// The offset to calc the position in mask texture.
  pub mask_offset: [f32; 2],
  /// The id of alpha mask texture, `load_color_primitives` method provide all
  /// textures a draw phase need.
  pub mask_id: u32,
  /// just use to help the struct memory align to 32 bytes
  pub(crate) _dummy: u32,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct TexturePrimitive {
  /// the transform for the sampler.
  pub transform: [[f32; 2]; 3],
  /// The origin of content box.
  pub content_origin: [f32; 2],
  /// The offset to calc the position in mask texture.
  pub mask_offset: [f32; 2],
  /// The origin of the brush placed in texture.
  pub brush_origin: [f32; 2],
  /// The size of the brush image.
  pub brush_size: [f32; 2],
  /// The index of texture, `load_color_primitives` method provide all textures
  /// a draw phase need.
  pub brush_tex_idx: u16,
  /// The id of alpha mask, `load_color_primitives` method provide all textures
  /// a draw phase need.
  pub mask_idx: u16,
  /// extra alpha apply to current vertex
  pub opacity: f32,
}
