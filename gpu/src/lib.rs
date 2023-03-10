pub mod error;
// #[cfg(feature = "wgpu_gl")]
// pub mod wgpu_gl;
// #[cfg(feature = "wgpu_gl")]
// pub use wgpu_gl::wgpu_backend_headless;
// #[cfg(feature = "wgpu_gl")]
// pub use wgpu_gl::wgpu_backend_with_wnd;
use guillotiere::Size;
use ribir_painter::DeviceRect;
use ribir_painter::TextureCfg;
use ribir_painter::TextureX;
use std::error::Error;
pub use tessellator::Tessellator;
pub mod tessellator;
use ribir_painter::image::ColorFormat;
use ribir_painter::DeviceSize;
use zerocopy::AsBytes;

// todo: use window size or monitor size to detect the cache size;
const ATLAS_SIZE: DeviceSize = DeviceSize::new(1024, 1024);
const CANVAS_SIZE: DeviceSize = DeviceSize::new(512, 512);

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub struct TextureId(pub usize);

pub trait GpuTessellatorHelper {
  type Texture: TextureX;

  /// The major texture used to display the final picture.
  fn main_texture(&mut self) -> &mut Self::Texture;

  /// Create a new texture, backend need to record the id will be used in
  /// primitive.
  fn new_texture(&mut self, id: TextureId, cfg: TextureCfg) -> Self::Texture;

  /// Set a clip rectangle and all triangles after this method called should
  /// clip by this rectangle.
  fn push_clip_rect(&mut self, rect: DeviceRect);

  /// Draw triangles to the texture,caller will try to batch as much as
  /// possible, but also possibly call multi times in a frame.
  fn draw_triangles(&mut self, texture: &mut Texture, data: TriangleLists);

  /// Draw triangles only alpha channel with 1.0. Caller guarantee the texture
  /// format is `ColorFormat::Alpha8`, caller will try to batch as much as
  /// possible, but also possibly call multi times in a frame.
  fn draw_alpha_triangles(
    &mut self,
    vertices: &[AlphaVertex],
    indices: &[u32],
    texture: &mut Texture,
  );

  /// cancel the last clip rectangle.
  fn pop_clip_rect(&mut self);
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
  /// indices range witch use pure color to draw. The primitive is
  /// `ColorPrimitive`
  Color(std::ops::Range<u32>),
  /// indices range witch use texture to draw. The primitive is
  /// `TexturePrimitive`
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
      transform,
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

// We use a texture atlas to shader vertices, even if a pure color path.
#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes, Default)]
pub struct AlphaVertex(f32, f32);

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
    p1 == p2
  }
}
