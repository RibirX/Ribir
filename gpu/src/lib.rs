pub mod error;
use std::ops::Range;

pub use gpu_backend::Texture;
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, Color, GradientStop, VertexBuffers};
mod gpu_backend;
use zerocopy::AsBytes;

#[cfg(feature = "wgpu")]
pub mod wgpu_impl;
pub use gpu_backend::*;
#[cfg(feature = "wgpu")]
pub use wgpu_impl::*;

/// Trait to help implement a gpu backend.
///
/// The call graph:
///
/// -- begin_frame()

///   +--->-------- Draw Phase --------------------------+
///   |                                                  |
///   |    +->- new_texture()----+                       |   
///   |    +-<-------<------<----+                       |
///   |                                                  v
///   | -> load_alpha_vertices()                         |
///   |                                                  |
///   | -> + draw_alpha_triangles_with_scissor()--+      |
///   |    ^                                      v      |
///   |    ^----<-----------<---------------------+      |
///   |                                                  |
///   | -> + draw_alpha_triangles()---------------+      |
///   |    ^                                      v      |
///   |    +----<-----------<---------------------+      |
///   |                                                  |
///   | -> load_textures()                               |
///   | -> load_mask_layers()                            |
///   |                                                  |    
///   |        +--------------------------+              |
///   |        |  load_color_vertices()   |              |
///   |     +->|  draw_color_triangles()  |              |
///   |     |  +--------------------------+              |
///   |     |                                            |
///   |     |  +--------------------------+              |
///   |     |  | load_img_primitives()    |              |
///   |     +->| load_image_vertices()    |              |
///   |     |  | draw_img_triangles()     |              |
///   |     |  +--------------------------+              |
///   |     |                                            |
///   | ->  |  +------------------------------------+    |
///   |     |  | load_radial_gradient_primitives()  |    v
///   |     +->| load_radial_gradient_stops()       |    |
///   |     |  | load_radial_gradient_vertices()    |    |
///   |     |  | draw_radial_gradient_triangles()   |    |
///   |     |  +------------------------------------+    |
///   |     |                                            |
///   |     |  +------------------------------------+    |
///   |     |  | load_linear_gradient_primitives()  |    |
///   |     +->| load_linear_gradient_stops()       |    |
///   |        | load_linear_gradient_vertices()    |    |
///   |        | draw_linear_gradient_triangles()   |    |
///   |        +------------------------------------+    |
///   +---<----------------------------------------------+
///
/// -+ ->- end_frame()
///
/// The coordinate always start from the left-top to right-bottom. Vertices
/// use percent as value, and others use pixel value.
///     Vertices Axis           Device axis
///  0  +----x----+> 1       0 +----x-----+> width
///     |         |            |          |   
///     y         |            y          |
///     |         |            |          |
///     +---------+            +----------+
///     v                      v
///     1                     height

pub trait GPUBackendImpl {
  type Texture: Texture;

  /// A frame start, call once per frame
  fn begin_frame(&mut self);

  /// Returns the limits of the GPU backend.
  fn limits(&self) -> &DrawPhaseLimits;

  /// Create a texture.
  fn new_texture(&mut self, size: DeviceSize, format: ColorFormat) -> Self::Texture;
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
    &mut self, indices: &Range<u32>, texture: &mut Self::Texture, scissor: DeviceRect,
  );

  /// load textures that will be use in this draw phase
  fn load_textures(&mut self, textures: &[&Self::Texture]);
  /// load the mask layers that the current draw phase will use, called at
  /// most once per draw phase.
  fn load_mask_layers(&mut self, layers: &[MaskLayer]);
  /// Load the vertices and indices buffer that `draw_color_triangles` will
  /// use.
  fn load_color_vertices(&mut self, buffers: &VertexBuffers<ColorAttr>);
  /// Load the vertices and indices buffer that `draw_img_triangles` will use.
  fn load_img_primitives(&mut self, primitives: &[ImgPrimitive]);
  /// Load the vertices and indices buffer that `draw_img_triangles` will use.
  fn load_img_vertices(&mut self, buffers: &VertexBuffers<ImagePrimIndex>);

  /// Load the primitives that `draw_radial_gradient_triangles` will use.
  fn load_radial_gradient_primitives(&mut self, primitives: &[RadialGradientPrimitive]);
  /// Load the gradient color stops that `draw_radial_gradient_triangles` will
  /// use.
  fn load_radial_gradient_stops(&mut self, stops: &[GradientStopPrimitive]);
  /// Load the vertices and indices buffer that `draw_radial_gradient_triangles`
  /// will use.
  fn load_radial_gradient_vertices(&mut self, buffers: &VertexBuffers<RadialGradientPrimIndex>);

  /// Load the primitives that `draw_linear_gradient_triangles` will use.
  fn load_linear_gradient_primitives(&mut self, primitives: &[LinearGradientPrimitive]);
  /// Load the gradient color stops that `draw_linear_gradient_triangles` will
  /// use.
  fn load_linear_gradient_stops(&mut self, stops: &[GradientStopPrimitive]);
  /// Load the vertices and indices buffer that `draw_linear_gradient_triangles`
  /// will use.
  fn load_linear_gradient_vertices(&mut self, buffers: &VertexBuffers<LinearGradientPrimIndex>);
  /// Draw pure color triangles in the texture. And use the clear color clear
  /// the texture first if it's a Some-Value
  fn draw_color_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  );
  /// Draw triangles fill with image. And use the clear color clear the texture
  /// first if it's a Some-Value
  fn draw_img_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  );
  /// Draw triangles fill with color radial gradient. And use the clear color
  /// clear the texture first if it's a Some-Value
  fn draw_radial_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  );

  /// Draw triangles fill with color linear gradient. And use the clear color
  /// clear the texture first if it's a Some-Value
  fn draw_linear_gradient_triangles(
    &mut self, texture: &mut Self::Texture, indices: Range<u32>, clear: Option<Color>,
  );

  fn copy_texture_from_texture(
    &mut self, dist_tex: &mut Self::Texture, copy_to: DevicePoint, from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  );
  /// A frame end, call once per frame
  fn end_frame(&mut self);
}

/// Represents the sets of limits an GPU backend can provide in a single draw
pub struct DrawPhaseLimits {
  /// The maximum size of the texture that the backend can create.
  pub texture_size: DeviceSize,
  /// The maximum number of textures that the backend can load in a single draw
  pub max_tex_load: usize,
  /// The maximum number of mask layers that the backend can load in a single
  /// draw phase
  pub max_image_primitives: usize,
  /// The maximum number of radial gradient primitives that the backend can load
  /// in a single draw
  pub max_radial_gradient_primitives: usize,
  /// The maximum number of linear gradient primitives that the backend can load
  /// in a single draw
  pub max_linear_gradient_primitives: usize,
  /// The maximum number of gradient stops that the backend can load in a single
  /// draw phase
  pub max_gradient_stop_primitives: usize,
  /// The maximum number of mask layers that the backend can load in a single
  pub max_mask_layers: usize,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct ColorAttr {
  /// brush's Rgba color
  pub color: [u8; 4],
  /// The index of the head mask layer.
  pub mask_head: i32,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct ImagePrimIndex(u32);

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct RadialGradientPrimIndex(u32);

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct LinearGradientPrimIndex(u32);

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct GradientStopPrimitive {
  pub color: u32,
  pub offset: f32,
}

impl GradientStopPrimitive {
  fn new(stop: &GradientStop) -> Self {
    GradientStopPrimitive { color: stop.color.into_u32(), offset: stop.offset }
  }
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct RadialGradientPrimitive {
  /// A 2x3 column-major matrix, transform a vertex position to the texture
  /// position
  pub transform: [f32; 6],
  /// The color stop's start index
  pub stop_start: u32,
  /// The size of the color stop
  pub stop_cnt: u32,
  /// position of the start center
  pub start_center: [f32; 2],
  /// position of the end center
  pub end_center: [f32; 2],
  /// the radius of the start circle.
  pub start_radius: f32,
  /// the radius of the end circle.
  pub end_radius: f32,
  /// The index of the head mask layer.
  pub mask_head: i32,
  /// the spread method of the gradient. 0 for pad, 1 for reflect and 2
  /// for repeat
  pub spread: u32,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct LinearGradientPrimitive {
  /// A 2x3 column-major matrix, transform a vertex position to the texture
  /// position
  pub transform: [f32; 6],
  /// position of the start center
  pub start_position: [f32; 2],
  /// position of the end center
  pub end_position: [f32; 2],
  /// The color stop information, there are two parts:
  /// - The high 16-bit index represents the start index of the color stop.
  /// - The low 16-bit index represents the size of the color stop.
  pub stop: u32,
  /// A mix of two 16-bit values:
  /// - The high 16-bit index represents the head mask layer.
  /// - The low 16-bit represents the spread method of the gradient. 0 for pad,
  ///   1 for reflect and 2 for repeat
  pub mask_head_and_spread: i32,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct ImgPrimitive {
  /// A 2x3 column-major matrix, transform a vertex position to the image
  /// texture slice position.
  pub transform: [f32; 6],
  /// The origin of the image placed in texture.
  pub img_start: [f32; 2],
  /// The size of the image image.
  pub img_size: [f32; 2],
  /// This represents a mix of two 16-bit indices:
  /// - The high 16-bit index represents the head mask layer. It is an i16.
  /// - The low 16-bit index represents the texture. It is a u16.
  pub mask_head_and_tex_idx: i32,
  /// extra alpha apply to current vertex
  pub opacity: f32,
}

/// The mask layer describes an alpha channel layer that is used in the fragment
/// shader to sample the alpha channel and apply it to the color.
#[derive(AsBytes, Clone)]
#[repr(packed)]
pub struct MaskLayer {
  /// A 2x3 column-major matrix, transform a vertex position to its mask texture
  /// position.
  pub transform: [f32; 6],
  /// The min position this layer in the texture.
  pub min: [f32; 2],
  /// max min position this layer in the texture.
  pub max: [f32; 2],
  /// The index of the texture(alpha) that contained this layer,
  /// `load_textures` method provide all textures a draw phase need.
  pub mask_tex_idx: u32,
  /// The index of the previous mask layer needs to continue to be applied. The
  /// negative value means there isn't any more mask layer that needs to be
  /// applied.
  pub prev_mask_idx: i32,
}
