#![feature(decl_macro, test)]
pub mod error;

#[cfg(feature = "wgpu_gl")]
pub mod wgpu_gl;

use tessellator::Tessellator;
pub mod tessellator;
use painter::{CaptureCallback, DeviceSize, PainterBackend};

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
  fn submit<'a>(
    &mut self,
    commands: Vec<painter::PaintCommand>,
    frame_data: Option<
      Box<dyn for<'r> FnOnce(DeviceSize, Box<dyn Iterator<Item = &[u8]> + 'r>) + 'a>,
    >,
  ) -> Result<(), &str> {
    self.gl.begin_frame();
    self.tessellator.tessellate(&commands, &mut self.gl);
    self.gl.end_frame(frame_data)
  }

  #[inline]
  fn resize(&mut self, size: DeviceSize) { self.gl.resize(size) }
}

/// GlRender support draw triangles to the devices.
pub trait GlRender {
  /// A new frame begin.
  fn begin_frame(&mut self);

  /// Add a texture which this frame will use.
  fn add_texture(&mut self, texture: Texture);

  /// Commit the render data to gl, caller will try to as possible as batch all
  /// render data, but also possible call `commit_render_data` multi time pre
  /// frame.
  fn draw_triangles(&mut self, data: TriangleLists);

  /// Draw frame finished and the render data commit finished and should ensure
  /// draw every of this frame into device. Call the `capture` callback to
  /// pass the frame image data with rgba(u8 x 4) format if it is Some-Value
  fn end_frame<'a>(&mut self, capture: Option<CaptureCallback<'a>>) -> Result<(), &str>;

  /// Window or surface size changed, need do a redraw.
  fn resize(&mut self, size: DeviceSize);
}

/// A texture for the vertexes sampler color. Every texture have identify to
/// help reuse gpu texture in adjacent frames. The `id` is a cycle increase
/// number, so it's always unique if the textures count is not over the
/// [`usize::MAX`]! in an application lifetime.
///
/// If the `id` is same with some texture of last frame, that mean they are the
/// same texture, in this case, provide `data` or not hint whether this texture
/// has changed.
///
/// For texture cache we only track the last frame, so if a texture use in frame
/// one and frame three but not use in frame two, it's have different `id` in
/// frame one and frame three.

pub struct Texture<'a> {
  /// The identify of the texture, unique in adjacent frames.
  pub id: usize,
  /// The texture size.
  pub size: (u16, u16),
  /// The data of the texture. A `None` value will give if the texture is not
  /// change to latest frame, so we can avoid to load the texture again.
  pub data: Option<&'a [u8]>,
  /// The color format of the texture
  pub format: ColorFormat,
}

pub enum DrawTriangles {
  /// indices range witch use pure color to draw.
  Color(std::ops::Range<u32>),
  /// indices range witch use texture to draw.
  Texture {
    rg: std::ops::Range<u32>,
    texture_id: usize,
  },
}

/// The triangle lists data and the commands to describe how to draw it.
pub struct TriangleLists<'a> {
  /// vertices buffer use to draw
  pub vertices: &'a [Vertex],
  /// indices buffer use to draw
  pub indices: &'a [u32],
  /// primitive use to interpretation scheme of the vertex
  pub primitives: &'a [Primitive],
  /// commands describe how to draw the indices.
  pub commands: &'a [DrawTriangles],
  // field to support clip and gradient
}

#[repr(C)]
#[derive(AsBytes, PartialEq, Clone)]
pub struct Primitive {
  // Both color and texture primitive have 128 bit size, see [`ColorPrimitive`]!  and
  // [`TexturePrimitive`]! to upstanding their struct.
  pub data: u128,
  /// the transform vertex to apply
  pub(crate) transform: [[f32; 2]; 3],
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
  /// Texture rect(x, y ,width, height) in texture, maybe placed in a
  /// atlas.
  pub(crate) tex_rect: [u16; 4],
  /// The factor use to calc the texture sampler position of vertex relative to
  /// the texture. Vertex calc its texture sampler pixel position across:
  /// vertex position multiplied by factor then modular texture size.
  ///
  /// - Repeat mode should be 1.
  /// - Cover mode should be  path.max / texture.size
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

impl<'a> TriangleLists<'a> {
  #[inline]
  pub fn is_empty(&self) -> bool { self.commands.is_empty() }
}

impl From<ColorPrimitive> for Primitive {
  #[inline]
  fn from(c: ColorPrimitive) -> Self { unsafe { std::mem::transmute(c) } }
}

impl From<TexturePrimitive> for Primitive {
  #[inline]
  fn from(t: TexturePrimitive) -> Self { unsafe { std::mem::transmute(t) } }
}
