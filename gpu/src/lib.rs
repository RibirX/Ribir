#![feature(decl_macro, test)]
pub mod error;

#[cfg(feature = "wgpu_gl")]
pub mod wgpu_gl;

use tessellator::Tessellator;
pub mod tessellator;
use painter::{DeviceSize, PainterBackend};

use painter::image::ColorFormat;
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
    self.tessellator.tessellate(&commands, |render_data| {
      self.gl.submit_render_data(render_data);
    });
    self.gl.finish()
  }

  #[inline]
  fn resize(&mut self, size: DeviceSize) { self.gl.resize(size) }
}
/// The Render that support draw the canvas result render data.
pub trait GlRender {
  /// Commit the render data to gl, caller will try to as possible as batch all
  /// render data, but also possible call `commit_render_data` multi time pre
  /// frame.
  fn submit_render_data(&mut self, data: RenderData);

  /// Window all surface size change, need do a redraw.
  fn resize(&mut self, size: DeviceSize);

  /// The render data commit finished.
  fn finish(&mut self);
}

/// A texture for the vertexes sampler color. Every texture have identify to
/// help reuse gpu texture in adjacent frames. The `id` is a cycle increase
/// number, so it's always unique if the textures count is not over the
/// [`usize::MAX`]! in an application lifetime.
///
/// For texture cache we only track the last frame, so if a texture use in frame
/// one and frame three but not use in frame two, it's have different `id` in
/// frame one and frame three.
pub struct Texture<'a> {
  /// The identify of the texture, unique in adjacent frames.
  pub id: usize,
  /// The texture size.
  pub size: DeviceSize,
  /// The data of the texture. A `None` value will give if the texture is not
  /// change to latest frame, should reuse the gpu texture.
  pub data: Option<&'a [u8]>,
  /// The color format of the texture
  pub format: ColorFormat,
}

/// Triangles with texture submit to gpu render
pub struct TextureRenderData<'a> {
  pub vertices: &'a [Vertex],
  pub indices: &'a [u32],
  /// Vertex extra info which contain the texture position of vertex and its
  /// transform matrix.
  pub primitives: &'a [TexturePrimitive],
  /// The texture store all the pixel color from.
  pub texture: Texture<'a>,
}

/// Triangles with color submit to gpu render
pub struct ColorRenderData<'a> {
  pub vertices: &'a [Vertex],
  pub indices: &'a [u32],
  /// Vertex extra info which contain the texture position of vertex and its
  /// transform matrix.
  pub primitives: &'a [ColorPrimitive],
}

pub enum RenderData<'a> {
  Color(ColorRenderData<'a>),
  Image(TextureRenderData<'a>),
}

#[repr(C)]
#[derive(AsBytes, PartialEq, Clone)]
pub struct ColorPrimitive {
  /// Rgba color
  pub(crate) color: [f32; 4],
  /// the transform vertex to apply
  pub(crate) transform: [[f32; 2]; 3],
}

#[repr(C)]
#[derive(AsBytes, PartialEq, Clone)]
pub struct TexturePrimitive {
  /// Texture offset in texture atlas.
  pub(crate) tex_offset: [u32; 2],
  /// The factor use to calc the texture sampler position of vertex. Vertex calc
  /// its texture sampler position (0..1) across:  vertex position multiplied
  /// by factor then keep fractional part of result.
  ///
  /// - Repeat mode should be  1 / sub_texture_size
  /// - Cover mode should be 1 / path_size
  pub(crate) factor: [f32; 2],
  /// the transform vertex to apply
  pub(crate) transform: [[f32; 2]; 3],
}

/// We use a texture atlas to shader vertices, even if a pure color path.
#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Vertex {
  pub pixel_coords: [f32; 2],
  pub prim_id: u32,
}

impl<'a> RenderData<'a> {
  #[inline]
  pub fn is_empty(&self) -> bool {
    match self {
      RenderData::Color(c) => c.indices.is_empty(),
      RenderData::Image(i) => i.indices.is_empty(),
    }
  }
}
