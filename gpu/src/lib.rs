pub mod error;
pub use gpu_backend::Texture;
use ribir_geom::{DevicePoint, DeviceRect, DeviceSize};
use ribir_painter::{image::ColorFormat, AntiAliasing, Color, VertexBuffers};
use std::ops::Range;
mod gpu_backend;
use zerocopy::AsBytes;

#[cfg(feature = "wgpu")]
pub mod wgpu_impl;
#[cfg(feature = "wgpu")]
pub use wgpu_impl::*;

pub use gpu_backend::*;

/// Trait to help implement a gpu backend.
///
/// The call graph:
///
/// -- begin_frame()

///   |
///   |    +->- new_texture()----+                          
///   |    +-<-------<------<----+
///   |
///   | -> load_alpha_vertices()
///   |
///   | -> + draw_alpha_triangles_with_scissor()--+    
///   |    ^                                      v
///   |    ^----<-----------<---------------------+    
///   |                                                
///   | -> + draw_alpha_triangles()---------------+    
///   |    ^                                      v
///   |    +----<-----------<---------------------+  
///   |
///   | -> load_mask_layers()
///   | -> load_textures()                             
///   | -> load_img_primitives()
///   | -> load_color_vertices()                   
///   | -> load_image_vertices()                     
///   |                                                                
///   |         +--- draw_color_triangles()----+            
///   |      +->+                              +---+        
///   |      |  +--- draw_img_triangles()------+   |        
///   |      +------<------------------------------+        
///   |
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

  /// Create a texture.
  fn new_texture(
    &mut self,
    size: DeviceSize,
    anti_aliasing: AntiAliasing,
    format: ColorFormat,
  ) -> Self::Texture;
  /// Load the vertices and indices buffer that `draw_alpha_triangles` &
  /// `draw_alpha_triangles_with_scissor` will use.
  fn load_alpha_vertices(&mut self, buffers: &VertexBuffers<f32>);
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
  fn load_img_vertices(&mut self, buffers: &VertexBuffers<u32>);

  /// Load the primitives that `draw_radial_gradient_triangles` will use.
  fn load_radial_gradient_primitives(&mut self, primitives: &[RadialGradientPrimitive]);
  /// Load the gradient color stops that `draw_radial_gradient_triangles` will
  /// use.
  fn load_radial_gradient_stops(&mut self, stops: &[GradientStopPrimitive]);
  /// Load the vertices and indices buffer that `draw_radial_gradient_triangles`
  /// will use.
  fn load_radial_gradient_vertices(&mut self, buffers: &VertexBuffers<RadialGradientAttr>);
  /// Draw pure color triangles in the texture. And use the clear color clear
  /// the texture first if it's a Some-Value
  fn draw_color_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  );
  /// Draw triangles fill with image. And use the clear color clear the texture
  /// first if it's a Some-Value
  fn draw_img_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  );
  /// Draw triangles fill with color radial gradient. And use the clear color
  /// clear the texture first if it's a Some-Value
  fn draw_radial_gradient_triangles(
    &mut self,
    texture: &mut Self::Texture,
    indices: Range<u32>,
    clear: Option<Color>,
  );

  fn copy_texture_from_texture(
    &mut self,
    dist_tex: &mut Self::Texture,
    copy_to: DevicePoint,
    from_tex: &Self::Texture,
    from_rect: &DeviceRect,
  );
  /// A frame end, call once per frame
  fn end_frame(&mut self);
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
pub struct RadialGradientAttr {
  pub prim_idx: u32,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct GradientStopPrimitive {
  pub red: f32,
  pub green: f32,
  pub blue: f32,
  pub alpha: f32,
  pub offset: f32,
}

#[repr(u32)]
enum SpreadMethod {
  Pad,
  Reflect,
  Repeat,
}

#[repr(packed)]
#[derive(AsBytes, PartialEq, Clone, Copy)]
pub struct RadialGradientPrimitive {
  /// A 2x3 column-major matrix, transform a vertex position to the radial path
  /// position
  pub transform: [f32; 6],
  /// The origin of the image placed in texture.
  pub stop_start: u32,
  /// The size of the image image.
  pub stop_cnt: u32,
  /// The index of texture, `load_color_primitives` method provide all textures
  /// a draw phase need.
  pub start_center: [f32; 2],
  /// The index of the head mask layer.
  pub end_center: [f32; 2],

  pub start_radius: f32,

  pub end_radius: f32,

  pub mask_head: i32,

  pub spread: u32,
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
  /// The index of texture, `load_color_primitives` method provide all textures
  /// a draw phase need.
  pub img_tex_idx: u32,
  /// The index of the head mask layer.
  pub mask_head: i32,
  /// extra alpha apply to current vertex
  pub opacity: f32,
  /// keep align to 8 bytes.
  _dummy: u32,
}

/// The mask layer describes an alpha channel layer that is used in the fragment
/// shader to sample the alpha channel and apply it to the color.
#[derive(AsBytes)]
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
  /// `load_color_primitives` method provide all textures a draw phase need.
  pub mask_tex_idx: u32,
  /// The index of the previous mask layer needs to continue to be applied. The
  /// negative value means there isn't any more mask layer that needs to be
  /// applied.
  pub prev_mask_idx: i32,
}
