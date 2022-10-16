#![feature(decl_macro, test)]
pub mod error;

#[cfg(feature = "wgpu_gl")]
pub mod wgpu_gl;

use std::error::Error;

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
  fn submit<'a>(&mut self, commands: Vec<painter::PaintCommand>) {
    self.gl.begin_frame();
    self.tessellator.tessellate(&commands, &mut self.gl);
    self.gl.end_frame(false);
  }

  #[inline]
  fn resize(&mut self, size: DeviceSize) { self.gl.resize(size) }

  fn commands_to_image(
    &mut self,
    commands: Vec<painter::PaintCommand>,
    capture: CaptureCallback,
  ) -> Result<(), Box<dyn Error>> {
    self.gl.begin_frame();
    self.tessellator.tessellate(&commands, &mut self.gl);
    self.gl.capture(capture)?;
    self.gl.end_frame(true);
    Ok(())
  }
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

  /// Capture the current frame image data, the `capture` callback will be
  /// called to pass the frame image data with rgba(u8 x 4) format.
  /// # Note
  /// Only capture stuff of the frame not ended, so should always call this
  /// method after `draw_triangles` and before `end_frame`.
  fn capture(&self, capture: CaptureCallback) -> Result<(), Box<dyn Error>>;

  /// Draw frame finished and the render data commit finished and should ensure
  /// draw every of this frame into device. Cancel current frame if `cancel` is
  /// true.
  fn end_frame<'a>(&mut self, cancel: bool);

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

  PushStencil(std::ops::Range<u32>),

  PopStencil(std::ops::Range<u32>),
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
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct ColorPrimitive {
  /// brush's Rgba color
  color: [f32; 4],
  /// the transform vertex to apply
  transform: [[f32; 2]; 3],
  /// extra alpha apply to current vertex
  opacity: f32,

  /// let the TexturePrimitive align to 16
  /// the alignment of the struct must restrict to https://www.w3.org/TR/WGSL/#alignment
  dummy: f32,
}

impl ColorPrimitive {
  fn new(color: [f32; 4], transform: [[f32; 2]; 3], opacity: f32) -> Self {
    Self {
      color,
      transform,
      opacity,
      dummy: 0.0,
    }
  }
}

#[repr(C)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct TexturePrimitive {
  /// Texture rect(x, y ,width, height) in texture, maybe placed in a
  /// atlas.
  tex_rect: [u16; 4],
  /// The factor use to calc the texture sampler position of vertex relative to
  /// the texture. Vertex calc its texture sampler pixel position across:
  /// vertex position multiplied by factor then modular texture size.
  ///
  /// - Repeat mode should be 1.
  /// - Cover mode should be  path.max / texture.size
  factor: [f32; 2],

  /// the transform vertex to apply
  transform: [[f32; 2]; 3],
  /// extra alpha apply to current vertex
  opacity: f32,

  /// let the TexturePrimitive align to 16
  /// the alignment of the struct must restrict to https://www.w3.org/TR/WGSL/#alignment
  dummy: f32,
}

impl TexturePrimitive {
  fn new(tex_rect: [u16; 4], factor: [f32; 2], transform: [[f32; 2]; 3], opacity: f32) -> Self {
    Self {
      tex_rect,
      factor,
      transform,
      opacity,
      dummy: 0.,
    }
  }
}

#[repr(C)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct StencilPrimitive {
  /// the transform vertex to apply
  transform: [[f32; 2]; 3],

  /// let the StencilPrimitive algin to Primitive
  dummy: [u32; 6],
}

impl StencilPrimitive {
  fn new(transform: [[f32; 2]; 3]) -> Self {
    StencilPrimitive {
      transform: transform,
      dummy: <[u32; 6]>::default(),
    }
  }
}

#[repr(C)]
#[derive(AsBytes, Clone, Copy)]
pub union Primitive {
  color_primitive: ColorPrimitive,
  texture_primitive: TexturePrimitive,
  stencil_primitive: StencilPrimitive,
}

/// We use a texture atlas to shader vertices, even if a pure color path.
#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes, Default)]
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
  fn from(c: ColorPrimitive) -> Self { Self { color_primitive: c } }
}

impl From<TexturePrimitive> for Primitive {
  #[inline]
  fn from(t: TexturePrimitive) -> Self { Self { texture_primitive: t } }
}

impl From<StencilPrimitive> for Primitive {
  #[inline]
  fn from(s: StencilPrimitive) -> Self { Self { stencil_primitive: s } }
}

impl PartialEq for Primitive {
  fn eq(&self, other: &Self) -> bool {
    const SIZE: usize = std::mem::size_of::<Primitive>();
    let p1: &[u8; SIZE] = unsafe { std::mem::transmute(self) };
    let p2: &[u8; SIZE] = unsafe { std::mem::transmute(other) };
    return p1 == p2;
  }
}
