use super::{
  atlas::TextureAtlas,
  mem_texture::MemTexture,
  tessellator_2d::Tessellator,
  text_brush::{Section, TextBrush},
  Command, CommandInfo, DeviceRect, DeviceSize, FillStyle, HorizontalAlign, Point, Rect,
  Rendering2DLayer, Size, Text, TextLayout, Transform, VerticalAlign,
};

use lyon::path::Path;
use lyon::tessellation::VertexBuffers;

use zerocopy::AsBytes;

/// Frame is a easy way to batch all layers to a frame and consume them and
/// batch submit to render.
pub struct Frame<'a, R: CanvasRender> {
  canvas: &'a mut Canvas,
  composed: Vec<Rendering2DLayer<'a>>,
  render: &'a mut R,
}

pub struct CanvasOptions {
  pub default_transform: Transform,
  pub texture_max_size: DeviceSize,
  pub texture_init_size: DeviceSize,
}
pub struct Canvas {
  // texture atlas for pure color and image to draw.
  atlas: TextureAtlas,
  glyph_brush: TextBrush,
  options: CanvasOptions,
  render_data: RenderData,
  default_transform: Transform,
}

/// An identify of a layer, generated by a layer composed. Use this to fast
/// compose the same layer after the first compose.
pub struct LayerId;

/// The Render that support draw the canvas result render data.
pub trait CanvasRender {
  fn draw(
    &mut self,
    data: &RenderData,
    glyph_texture: &mut MemTexture<u8>,
    atlas_texture: &mut MemTexture<u32>,
  );

  fn resize(&mut self, size: DeviceSize);
}

impl Canvas {
  pub fn new(options: Option<CanvasOptions>) -> Self {
    let options = options.unwrap_or_default();
    let CanvasOptions {
      texture_init_size: init_size,
      texture_max_size: max_size,
      default_transform,
    } = options;
    Self {
      atlas: TextureAtlas::new(options.texture_init_size, options.texture_max_size),
      glyph_brush: TextBrush::new(init_size, max_size),
      options,
      render_data: RenderData::new(),
      default_transform,
    }
  }

  #[inline]
  pub fn options(&self) -> &CanvasOptions { &self.options }

  /// The brush canvas use to draw text, can use it to mange fonts.
  #[inline]
  pub fn text_brush(&mut self) -> &mut TextBrush { &mut self.glyph_brush }

  /// Create a 2d layer for drawing, and not effect canvas visual before consume
  /// by canvas and submit to render.
  pub fn new_2d_layer<'l>(&self) -> Rendering2DLayer<'l> {
    let mut layer = Rendering2DLayer::new();
    layer.set_transform(self.default_transform);
    layer
  }

  /// Create a frame to auto batch your layers, and submit all layers when frame
  /// drop.
  pub fn next_frame<'a, R: CanvasRender>(&'a mut self, render: &'a mut R) -> Frame<'a, R> {
    Frame {
      render,
      canvas: self,
      composed: vec![],
    }
  }

  /// Cache the layer in canvas, and return a `LayerId` to identify it. If a
  /// layer will compose many time, use layer id to compose it will have better
  /// performance.
  pub fn cache_layer(&mut self, _layer: Rendering2DLayer) -> LayerId {
    unimplemented!();
  }

  /// Consume all composed layer but not draw yet, then submit the output to
  /// render to draw.
  pub fn submit<R: CanvasRender>(&mut self, render: &mut R) {
    self.submit_to_render(render);
    self.render_data.clear();
  }

  pub fn mesure_text<'a>(&mut self, src: &'a Text) -> Rect {
    let text = src.to_glyph_text(self.text_brush(), 0);
    let mut sec = Section::new().add_text(text);
    sec.bounds = (f32::INFINITY, f32::INFINITY);
    self
      .text_brush()
      .section_bounds(&sec)
      .unwrap_or_else(Rect::zero)
  }

  #[inline]
  pub fn default_transform(&self) -> Transform { self.default_transform }

  #[inline]
  pub fn set_default_transform(&mut self, transform: Transform) {
    self.default_transform = transform;
  }

  #[inline]
  pub(crate) fn render_data(&self) -> &RenderData { &self.render_data }

  pub(crate) fn store_style_in_atlas<R: CanvasRender>(
    &mut self,
    style: &FillStyle,
    render: &mut R,
  ) -> DeviceRect {
    match style {
      FillStyle::Color(c) => {
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

  /// The behavior like [`Canvas::process_queued`](Canvas::process_queued), but
  /// if the texture cache is not enough, will submit render data in canvas and
  /// try expand the texture cache.
  pub(crate) fn process_queued_with_render<R: CanvasRender>(&mut self, render: &mut R) -> bool {
    let mut split_draw = false;

    loop {
      match self
        .glyph_brush
        .process_queued(&mut self.render_data.vertices_buffer)
      {
        Ok(_) => break,
        Err(glyph_brush::BrushError::TextureTooSmall { .. }) => {
          if !split_draw {
            split_draw = true;
            self.submit_to_render(render);
            // only clear the vertices buffer, but leave primitives to keep the primitive id
            // already in queued text.
            self.render_data.vertices_buffer.vertices.clear();
            self.render_data.vertices_buffer.indices.clear();
          }
          self.text_brush().grow_texture();
        }
      }
    }
    split_draw
  }

  #[inline]
  pub fn atlas(&self) -> &TextureAtlas { &self.atlas }

  pub fn consume_2d_layer<R: CanvasRender>(
    &mut self,
    layer: Rendering2DLayer,
    tessellator: &mut Tessellator,
    render: &mut R,
  ) {
    fn font_device_scale(mut transform: Transform) -> (f32, Transform) {
      let scale = transform.m11.max(transform.m22);
      if scale > f32::EPSILON {
        let s = 1. / scale;
        transform = transform.pre_scale(s, s);
      }
      (scale, transform)
    }

    layer
      .commands
      .into_iter()
      .for_each(|Command { transform, info }| {
        match info {
          CommandInfo::Path { path, style, stroke_width } => {
            let style_rect = self.store_style_in_atlas(&style, render);
            let align_bounds = path_bounds_to_align_texture(&style, &path.0);
            self.add_primitive(style_rect, align_bounds, transform);
            let prim_id = self.render_data.primitives.len() as u32 - 1;
            let vertices_buffer = &mut self.render_data.vertices_buffer;
            tessellator.tessellate(vertices_buffer, path.0, stroke_width, &transform, prim_id);
          }
          CommandInfo::SimpleText { text, style, max_width } => {
            let (scale, transform) = font_device_scale(transform);
            let text = text
              .to_glyph_text(self.text_brush(), 0)
              .with_scale(text.font_size * scale);
            let mut sec = Section::new().add_text(text);
            if let Some(max_width) = max_width {
              sec.bounds = (max_width, f32::INFINITY)
            }
            let align_bounds = section_bounds_to_align_texture(self.text_brush(), &style, &sec);
            if !align_bounds.is_empty() {
              self.single_style_section_consume(&style, render, align_bounds, transform, sec);
            }
          }
          CommandInfo::ComplexTexts { texts, bounds, layout } => {
            let (scale, transform) = font_device_scale(transform);
            let texts = texts
              .into_iter()
              .map(|(t, color)| {
                let style_rect = self.store_style_in_atlas(&color.into(), render);
                self.add_primitive(style_rect, COLOR_BOUNDS_TO_ALIGN_TEXTURE, transform);
                let prim_id = self.render_data.primitives.len() - 1;
                t.to_glyph_text(self.text_brush(), prim_id)
                  .with_scale(t.font_size * scale)
              })
              .collect();

            let mut sec = Section::new().with_text(texts);
            sec = section_with_layout_bounds(sec, bounds, layout);
            self.consume_section(render, sec);
          }
          CommandInfo::ComplexTextsByStyle { style, texts, bounds, layout } => {
            let (scale, transform) = font_device_scale(transform);
            let texts = texts
              .into_iter()
              .map(|t| {
                t.to_glyph_text(self.text_brush(), 0)
                  .with_scale(t.font_size * scale)
              })
              .collect();
            let mut sec = Section::new().with_text(texts);
            let align_bounds = section_bounds_to_align_texture(self.text_brush(), &style, &sec);
            if !align_bounds.is_empty() {
              sec = section_with_layout_bounds(sec, bounds, layout);
              self.single_style_section_consume(&style, render, align_bounds, transform, sec);
            }
          }
        };
      });
  }

  fn single_style_section_consume<R: CanvasRender>(
    &mut self,
    style: &FillStyle,
    render: &mut R,
    align_bounds: Rect,
    transform: Transform,
    mut sec: Section,
  ) {
    let style_rect = self.store_style_in_atlas(style, render);
    self.add_primitive(style_rect, align_bounds, transform);
    let prim_id = self.render_data.primitives.len() as u32 - 1;
    sec.text.iter_mut().for_each(|t| t.extra = prim_id);
    self.consume_section(render, sec);
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

  fn consume_section<R: CanvasRender>(&mut self, render: &mut R, sec: Section) {
    self.text_brush().queue(sec);
    self.process_queued_with_render(render);
  }

  fn submit_to_render<R: CanvasRender>(&mut self, render: &mut R) {
    if self.render_data().has_data() {
      render.draw(
        &self.render_data,
        self.glyph_brush.texture_mut(),
        self.atlas.texture_mut(),
      )
    }
  }
}

impl<'a, R: CanvasRender> Frame<'a, R> {
  /// Compose a layer to this frame.
  #[inline]
  pub fn compose_2d_layer(&mut self, layer: Rendering2DLayer<'a>) { self.composed.push(layer); }

  #[inline]
  pub fn new_2d_layer(&self) -> Rendering2DLayer { self.canvas.new_2d_layer() }

  /// Compose the `id` represented layer into frame. `LayerId` generate by
  /// [`cache_layer`](Canvas::cache_layer).
  pub fn compose_layer_by_id(&mut self, _id: LayerId) {
    unimplemented!();
  }
}

impl<'a, R: CanvasRender> Drop for Frame<'a, R> {
  fn drop(&mut self) {
    let Self { canvas, composed, render } = self;
    let mut tessellator = crate::tessellator_2d::Tessellator::new();
    composed
      .drain(..)
      .for_each(|layer| canvas.consume_2d_layer(layer, &mut tessellator, *render));
    self.canvas.submit(self.render);
  }
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
  fn new() -> RenderData {
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

  fn clear(&mut self) {
    self.vertices_buffer.vertices.clear();
    self.vertices_buffer.indices.clear();
    self.primitives.clear();
  }
}

impl Default for CanvasOptions {
  fn default() -> Self {
    Self {
      texture_init_size: DeviceSize::new(1024, 1024),
      texture_max_size: DeviceSize::new(4096, 4096),
      default_transform: Transform::new(1., 0., 0., 1., 0., 0.),
    }
  }
}

fn section_with_layout_bounds(
  mut sec: Section,
  bounds: Option<Rect>,
  layout: Option<TextLayout>,
) -> Section {
  if let Some(layout) = layout {
    sec = sec.with_layout(layout);
  }
  if let Some(bounds) = bounds {
    sec = section_with_bounds(sec, bounds);
  }
  sec
}

fn section_with_bounds(mut sec: Section, bounds: Rect) -> Section {
  sec = sec.with_bounds(bounds.size);

  let (h_align, v_align) = match &sec.layout {
    glyph_brush::Layout::SingleLine { h_align, v_align, .. } => (h_align, v_align),
    glyph_brush::Layout::Wrap { h_align, v_align, .. } => (h_align, v_align),
  };

  let mut pos = bounds.min();
  match h_align {
    HorizontalAlign::Left => {}
    HorizontalAlign::Center => pos.x = bounds.center().x,
    HorizontalAlign::Right => pos.x = bounds.max_x(),
  }
  match v_align {
    VerticalAlign::Top => {}
    VerticalAlign::Center => pos.y = bounds.center().y,
    VerticalAlign::Bottom => pos.y = bounds.max_y(),
  }
  sec.with_screen_position(pos)
}

// Pure color just one pixel in texture, and always use repeat pattern, so
// zero min is ok, no matter what really bounding it is.
const COLOR_BOUNDS_TO_ALIGN_TEXTURE: Rect = Rect::new(Point::new(0., 0.), Size::new(1., 1.));

fn path_bounds_to_align_texture(style: &FillStyle, path: &Path) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    let rect = lyon::algorithms::aabb::bounding_rect(path.iter());
    Rect::from_untyped(&rect)
  }
}

fn section_bounds_to_align_texture(
  text_brush: &mut TextBrush,
  style: &FillStyle,
  sec: &Section,
) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    text_brush.section_bounds(sec).unwrap_or_else(Rect::zero)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;

  #[test]
  fn bounding_align() {
    let mut path = PathBuilder::new();
    path.rect(&Rect::new(Point::new(100., 100.), Size::new(50., 50.)));
    let path = path.build();

    let rect = path_bounds_to_align_texture(&Color::BLACK.into(), &path.0);
    assert_eq!(rect, Rect::from_size(Size::new(1., 1.)));

    let rect = path_bounds_to_align_texture(&FillStyle::Image, &path.0);
    assert_eq!(rect.min(), Point::new(100., 100.));
    assert_eq!(rect.size, Size::new(50., 50.));
  }

  #[test]
  fn section_bounds_layout() {
    let section = Section::new();
    let rect = euclid::rect(10., 20., 40., 30.);
    let layout = TextLayout::default();

    let l = layout.clone();
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    assert_eq!(s.screen_position, rect.min().into());
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.center().x, rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Right;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.max_x(), rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.v_align = VerticalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.center().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout;
    l.v_align = VerticalAlign::Bottom;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.max_y());
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());
  }
}
