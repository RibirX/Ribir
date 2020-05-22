use crate::{canvas::surface::Surface, text::Section, Canvas, Point, Rect, Size, Transform};
pub use glyph_brush::{FontId, GlyphCruncher, HorizontalAlign, Layout, VerticalAlign};
pub use lyon::{
  path::{builder::PathBuilder, traits::PathIterator, Path, Winding},
  tessellation::*,
};
pub use palette::{named as const_color, Srgba};

use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut},
};

const TOLERANCE: f32 = 0.5;
pub type Color = Srgba<u8>;

/// The 2d layer is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
#[derive(Debug, Clone)]
pub struct Rendering2DLayer<'a> {
  state_stack: Vec<State>,
  commands: Vec<Command<'a>>,
}

impl<'a> Rendering2DLayer<'a> {
  pub(crate) fn new() -> Self {
    Self {
      state_stack: vec![State::new()],
      commands: vec![],
    }
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
  #[must_use]
  pub fn save<'l>(&'l mut self) -> LayerGuard<'l, 'a> {
    let new_state = self.current_state().clone();
    self.state_stack.push(new_state);
    LayerGuard(self)
  }

  /// Returns the color, gradient, or pattern used for draw. Only `Color`
  /// support now.
  #[inline]
  pub fn get_style(&self) -> &FillStyle { &self.current_state().style }

  /// Change the style of pen that used to draw path.
  #[inline]
  pub fn set_style(&mut self, pen_style: FillStyle) -> &mut Self {
    self.current_state_mut().style = pen_style;
    self
  }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn get_line_width(&self) -> f32 { self.current_state().line_width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().line_width = line_width;
    self
  }

  #[inline]
  pub fn get_font(&self) -> FontId { self.current_state().font }

  #[inline]
  pub fn set_font(&mut self, font: FontId) -> &mut Self {
    self.current_state_mut().font = font;
    self
  }

  /// Return the current transformation matrix being applied to the layer.
  #[inline]
  pub fn get_transform(&self) -> &Transform { &self.current_state().transform }

  /// Resets (overrides) the current transformation to the identity matrix, and
  /// then invokes a transformation described by the arguments of this method.
  /// This lets you scale, rotate, translate (move), and skew the context.
  #[inline]
  pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
    self.current_state_mut().transform = transform;
    self
  }

  /// Renders the specified path by using the current pen.
  pub fn stroke_path(&mut self, path: Path) {
    let cmd = self.command_from_path(path, true);
    self.commands.push(cmd);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let cmd = self.command_from_path(path, false);
    self.commands.push(cmd);
  }

  /// Fill `text` from left to right, start at `left_top`.
  /// Partially hitting the `max_width` will end the draw.
  /// Use `font` and `font_size` to specify the font and font size.
  /// Use [`fill_text_with_desc`](Rendering2DLayer::fill_text_with_desc) method
  /// to fill complex text.
  pub fn fill_text(&mut self, left_top: Point, text: &'a str, max_width: Option<f32>) {
    let cmd = self.command_from_text(text, left_top, max_width);
    self.commands.push(cmd);
  }

  /// Draw multi texts with different font and color, and specify how to layout
  /// it. Use [`fill_text`](Rendering2DLayer::fill_text) if just draw a single
  /// line simple text.
  ///
  /// # Arguments.
  ///
  /// * `texts` -  Pairs of Text and its color to render, rendered next to one
  ///   another.
  /// * `bounds` - Box bounds, in pixels from top-left.
  /// * `layout` - Layout info of the texts
  pub fn fill_complex_texts(
    &mut self,
    texts: Vec<(Text<'a>, Color)>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let cmd = self.command_from(|_| CommandInfo::ComplexTexts {
      texts,
      bounds,
      layout,
    });
    self.commands.push(cmd)
  }

  /// Draw multi texts with different font, and specify how to layout it. Its
  /// behavior is similar with
  /// [`fill_complex_texts`](Rendering2DLayer::fill_complex_texts), but use
  /// current style to draw and texts can't specify color.
  ///
  /// # Arguments.
  ///
  /// * `texts` -  texts to render, rendered next to one another.
  /// * `bounds` - Box bounds, in pixels from top-left.
  /// * `layout` - Layout info of the texts
  pub fn fill_complex_texts_by_style(
    &mut self,
    texts: Vec<Text<'a>>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let cmd = self.command_from(|state| CommandInfo::ComplexTextsByStyle {
      texts,
      bounds,
      layout,
      style: state.style.clone(),
    });
    self.commands.push(cmd)
  }

  /// Adds a translation transformation to the current matrix by moving the
  /// canvas and its origin x units horizontally and y units vertically on the
  /// grid.
  ///
  /// * `x` - Distance to move in the horizontal direction. Positive values are
  ///   to the right, and negative to the left.
  /// * `y` - Distance to move in the vertical direction. Positive values are
  ///   down, and negative are up.
  pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
    let t = &mut self.current_state_mut().transform;
    *t = t.post_translate(euclid::Vector2D::new(x, y));
    self
  }

  /// All drawing of this layer has finished, commit it to canvas and convert
  /// the layer to an intermediate render buffer data that will provide to
  /// render process and then commit to gpu.
  ///
  /// Return
  /// If the canvas texture cache is update in the process, will return a
  /// None-Value that means there is no buffer data can be cached.
  pub fn finish<S>(self, canvas: &mut Canvas<S>) -> Option<RenderCommand>
  where
    S: Surface,
  {
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    let mut geometry = VertexBuffers::new();
    let mut attrs: Vec<RenderAttr> = vec![];
    let mut unprocessed_attrs = vec![];
    let mut texture_updated = false;

    let mut last_text = false;
    self
      .commands
      .into_iter()
      .for_each(|Command { transform, info }| {
        match info {
          CommandInfo::Path {
            path,
            style,
            stroke_width,
          } => {
            if last_text {
              texture_updated = canvas.process_queued_text(&mut geometry);
              attrs.append(&mut unprocessed_attrs);
              last_text = false;
            }
            let count = if let Some(line_width) = stroke_width {
              stroke_tess
                .tessellate_path(
                  &path,
                  &StrokeOptions::tolerance(TOLERANCE).with_line_width(line_width),
                  &mut BuffersBuilder::new(&mut geometry, Vertex::from_stroke_vertex),
                )
                .unwrap()
            } else {
              fill_tess
                .tessellate_path(
                  &path,
                  &FillOptions::tolerance(TOLERANCE),
                  &mut BuffersBuilder::new(&mut geometry, Vertex::from_fill_vertex),
                )
                .unwrap()
            };
            let bounds = path_bounds_to_align_texture(&style, path);
            add_attr_and_try_merge(&mut attrs, count, transform, style, bounds);
          }
          CommandInfo::SimpleText {
            text,
            style,
            pos,
            max_width,
          } => {
            let mut sec = Section::new().add_text(text).with_screen_position(pos);
            if let Some(max_width) = max_width {
              sec.bounds = (max_width, f32::INFINITY).into()
            }
            let bounds = section_bounds_to_align_texture(canvas, &style, &sec);
            if !bounds.is_empty_or_negative() {
              let glyph_count = canvas.glyph_brush.glyphs(&sec).count() as u32;
              let count = glyphs_geometry_count(glyph_count);
              add_attr_and_try_merge(&mut unprocessed_attrs, count, transform, style, bounds);
            }
            canvas.queue(sec);
            last_text = true;
          }
          CommandInfo::ComplexTexts {
            texts,
            bounds,
            layout,
          } => {
            let texts = texts
              .into_iter()
              .map(|(t, color)| {
                let glyph_count = canvas
                  .glyph_brush
                  .glyphs(&Section::new().add_text(t.clone()))
                  .count() as u32;
                add_attr_and_try_merge(
                  &mut unprocessed_attrs,
                  glyphs_geometry_count(glyph_count),
                  transform,
                  FillStyle::Color(color),
                  COLOR_BOUNDS_TO_ALIGN_TEXTURE,
                );
                t.into()
              })
              .collect();
            let mut sec = Section::new().with_text(texts);
            if let Some(bounds) = bounds {
              sec = sec
                .with_screen_position(bounds.min())
                .with_bounds(bounds.size);
            }
            if let Some(layout) = layout {
              sec = sec.with_layout(layout);
            }
            canvas.queue(sec);
            last_text = true;
          }
          CommandInfo::ComplexTextsByStyle {
            style,
            texts,
            bounds,
            layout,
          } => {
            let texts = texts.into_iter().map(|t| t.into()).collect();
            let mut sec = Section::new().with_text(texts);
            let align_bounds = section_bounds_to_align_texture(canvas, &style, &sec);
            if !align_bounds.is_empty_or_negative() {
              if let Some(bounds) = bounds {
                sec = sec
                  .with_screen_position(bounds.min())
                  .with_bounds(bounds.size);
              }
              if let Some(layout) = layout {
                sec = sec.with_layout(layout);
              }

              let glyph_count = canvas.glyph_brush.glyphs(&sec).count() as u32;
              let count = glyphs_geometry_count(glyph_count);
              add_attr_and_try_merge(
                &mut unprocessed_attrs,
                count,
                transform,
                style,
                align_bounds,
              );
              canvas.queue(sec);
              last_text = true;
            }
          }
        };
      });

    if last_text {
      canvas.process_queued_text(&mut geometry);
      attrs.append(&mut unprocessed_attrs);
    }
    let cmd = RenderCommand { geometry, attrs };
    if texture_updated {
      canvas.upload_render_command(&cmd);
      None
    } else {
      Some(cmd)
    }
  }
}

/// Describe render the text as single line or break as multiple lines.
#[derive(Debug, Clone, PartialEq)]
pub enum LineWrap {
  /// Renders a single line from left-to-right according to the inner
  /// alignment. Hard breaking will end the line, partially hitting the width
  /// bound will end the line.
  SingleLine,
  /// Renders multiple lines from left-to-right according to the inner
  /// alignment. Hard breaking characters will cause advancement to another
  /// line. A characters hitting the width bound will also cause another line
  /// to start.
  Wrap,
}

/// Describes how to layout the text.
#[derive(Debug, Clone)]
pub struct TextLayout {
  /// horizontal alignment preference
  pub h_align: HorizontalAlign,
  /// vertical alignment preference
  pub v_align: VerticalAlign,
  /// text render in single line a multiple lins.
  pub wrap: LineWrap,
}

#[derive(Debug, Clone)]
pub struct Text<'a> {
  /// Text to render
  pub text: &'a str,
  /// Text pixel size.
  pub font_size: f32,
  /// It must be a valid id of font, can query font id from
  /// [`Canvas::get_font_id_by_name`](Canvas::get_font_id_by_name) or across
  /// canvas to load custom font The default `FontId(0)` should always be
  pub font_id: FontId,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderAttr {
  pub(crate) count: Count,
  pub(crate) transform: Transform,
  pub(crate) style: FillStyle,
  pub(crate) bounding_rect_for_style: Rect,
}
#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
  Color(Color),
  Image,    // todo
  Gradient, // todo,
}

#[derive(Debug, Clone)]
pub(crate) struct Vertex {
  pub(crate) pixel_coords: Point,
  pub(crate) texture_coords: Point,
}

#[derive(Debug, Clone)]
pub struct RenderCommand {
  pub(crate) geometry: VertexBuffers<Vertex, u32>,
  pub(crate) attrs: Vec<RenderAttr>,
}

impl<'a> Rendering2DLayer<'a> {
  fn current_state(&self) -> &State {
    self
      .state_stack
      .last()
      .expect("Must have one state in stack!")
  }

  fn current_state_mut(&mut self) -> &mut State {
    self
      .state_stack
      .last_mut()
      .expect("Must have one state in stack!")
  }

  fn command_from_path<'l>(&self, path: Path, stroke_or_fill: bool) -> Command<'l> {
    self.command_from(|state| {
      let stroke_width = if stroke_or_fill {
        Some(self.current_state().line_width)
      } else {
        None
      };
      CommandInfo::Path {
        path,
        style: state.style.clone(),
        stroke_width,
      }
    })
  }

  fn command_from_text<'l>(
    &self,
    text: &'l str,
    pos: Point,
    max_width: Option<f32>,
  ) -> Command<'l> {
    self.command_from(|state| CommandInfo::SimpleText {
      text: Text {
        text,
        font_size: state.font_size,
        font_id: state.font,
      },
      style: state.style.clone(),
      pos,
      max_width,
    })
  }

  #[inline]
  fn command_from<'l, F: FnOnce(&State) -> CommandInfo<'l>>(&self, ctor_info: F) -> Command<'l> {
    let state = self.current_state();
    Command {
      info: ctor_info(state),
      transform: state.transform,
    }
  }
}

fn add_attr_and_try_merge(
  attrs: &mut Vec<RenderAttr>,
  count: Count,
  transform: Transform,
  style: FillStyle,
  bounds: Rect,
) {
  if let Some(last) = attrs.last_mut() {
    if last.bounding_rect_for_style == bounds && last.style == style && last.transform == transform
    {
      last.count.vertices += count.vertices;
      last.count.indices += count.indices;
      return;
    }
  }

  attrs.push(RenderAttr {
    transform,
    bounding_rect_for_style: bounds,
    count,
    style: style.clone(),
  });
}

#[inline]
fn glyphs_geometry_count(glyph_count: u32) -> Count {
  Count {
    vertices: glyph_count * 4,
    indices: glyph_count * 6,
  }
}
impl<S: Surface> Canvas<S> {
  fn process_queued_text(&mut self, geometry: &mut VertexBuffers<Vertex, u32>) -> bool {
    let mut texture_updated = false;
    let quad_vertices = loop {
      match self.process_queued() {
        Ok(quad_vertices) => break quad_vertices,
        Err(glyph_brush::BrushError::TextureTooSmall { suggested }) => {
          self.submit();
          self.glyph_brush.resize_texture(&self.device, suggested);
          texture_updated = true;
        }
      };
    };

    let count = Count {
      vertices: quad_vertices.len() as u32 * 4,
      indices: quad_vertices.len() as u32 * 6,
    };
    geometry.vertices.reserve(count.vertices as usize);
    geometry.indices.reserve(count.indices as usize);

    fn rect_corners(rect: &Rect) -> [Point; 4] {
      [
        rect.min(),
        Point::new(rect.max_x(), rect.min_y()),
        Point::new(rect.min_x(), rect.max_y()),
        rect.max(),
      ]
    }
    quad_vertices.iter().for_each(|v| {
      let VertexBuffers { vertices, indices } = geometry;
      let offset = vertices.len() as u32;
      let tl = offset;
      let tr = 1 + offset;
      let bl = 2 + offset;
      let br = 3 + offset;
      indices.push(tl);
      indices.push(tr);
      indices.push(bl);
      indices.push(bl);
      indices.push(tr);
      indices.push(br);

      let px_coords = rect_corners(&v.pixel_coords);
      let tex_coords = rect_corners(&v.tex_coords);
      vertices.push(Vertex {
        pixel_coords: px_coords[0],
        texture_coords: tex_coords[0],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[1],
        texture_coords: tex_coords[1],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[2],
        texture_coords: tex_coords[2],
      });
      vertices.push(Vertex {
        pixel_coords: px_coords[3],
        texture_coords: tex_coords[3],
      });
    });
    texture_updated
  }
}
#[derive(Clone, Debug)]
struct State {
  transform: Transform,
  line_width: f32,
  style: FillStyle,
  font: FontId,
  font_size: f32,
}

#[derive(Debug, Clone)]
enum CommandInfo<'a> {
  Path {
    path: Path,
    style: FillStyle,
    // A some value means stroke with the line width in it, None means fill.
    stroke_width: Option<f32>,
  },
  SimpleText {
    text: Text<'a>,
    style: FillStyle,
    pos: Point,
    max_width: Option<f32>,
  },
  ComplexTexts {
    texts: Vec<(Text<'a>, Color)>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  },
  ComplexTextsByStyle {
    texts: Vec<Text<'a>>,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
    style: FillStyle,
  },
}

#[derive(Debug, Clone)]
struct Command<'a> {
  info: CommandInfo<'a>,
  transform: Transform,
}

/// An RAII implementation of a "scoped state" of the render layer. When this
/// structure is dropped (falls out of scope), changed state will auto restore.
/// The data can be accessed through this guard via its Deref and DerefMut
/// implementations.
pub struct LayerGuard<'a, 'b>(&'a mut Rendering2DLayer<'b>);

impl<'a, 'b> Drop for LayerGuard<'a, 'b> {
  #[inline]
  fn drop(&mut self) {
    debug_assert!(!self.0.state_stack.is_empty());
    self.0.state_stack.pop();
  }
}

impl<'a, 'b> Deref for LayerGuard<'a, 'b> {
  type Target = Rendering2DLayer<'b>;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a, 'b> DerefMut for LayerGuard<'a, 'b> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Vertex {
  fn from_stroke_vertex(v: StrokeVertex) -> Self {
    Self {
      pixel_coords: Point::from_untyped(v.position()),
      texture_coords: Point::new(-1., -1.),
    }
  }

  fn from_fill_vertex(v: FillVertex) -> Self {
    Self {
      pixel_coords: Point::from_untyped(v.position()),
      texture_coords: Point::new(-1., -1.),
    }
  }
}

// Pure color just one pixel in texture, and always use repeat pattern, so
// zero min is ok, no matter what really bounding it is.
const COLOR_BOUNDS_TO_ALIGN_TEXTURE: Rect = Rect::new(Point::new(0., 0.), Size::new(1., 1.));

fn path_bounds_to_align_texture(style: &FillStyle, path: Path) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    let rect = lyon::algorithms::aabb::bounding_rect(path.iter());
    Rect::from_untyped(&rect)
  }
}

fn section_bounds_to_align_texture<S: Surface>(
  canvas: &mut Canvas<S>,
  style: &FillStyle,
  sec: &Section,
) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    canvas.glyph_brush.glyph_bounds(sec).unwrap_or(Rect::zero())
  }
}

impl Default for FillStyle {
  #[inline]
  fn default() -> Self { FillStyle::Color(const_color::WHITE.into()) }
}

impl Default for State {
  #[inline]
  fn default() -> Self { Self::new() }
}

impl State {
  pub const fn new() -> Self {
    Self {
      transform: Transform::row_major(1., 0., 0., 1., 0., 0.),
      style: FillStyle::Color(Color {
        color: const_color::BLACK,
        alpha: u8::MAX,
      }),
      line_width: 1.,
      font: FontId(0),
      font_size: 14.,
    }
  }
}

impl Default for TextLayout {
  #[inline]
  fn default() -> Self { Self::new() }
}

impl TextLayout {
  const fn new() -> Self {
    Self {
      v_align: VerticalAlign::Center,
      h_align: HorizontalAlign::Left,
      wrap: LineWrap::SingleLine,
    }
  }
}

impl From<TextLayout> for glyph_brush::Layout<glyph_brush::BuiltInLineBreaker> {
  fn from(layout: TextLayout) -> Self {
    let TextLayout {
      h_align,
      v_align,
      wrap,
    } = layout;
    let line_breaker = glyph_brush::BuiltInLineBreaker::default();
    if LineWrap::SingleLine == wrap {
      glyph_brush::Layout::SingleLine {
        h_align,
        v_align,
        line_breaker,
      }
    } else {
      glyph_brush::Layout::Wrap {
        h_align,
        v_align,
        line_breaker,
      }
    }
  }
}

impl<'a> From<Text<'a>> for glyph_brush::Text<'a, ()> {
  fn from(text: Text<'a>) -> Self {
    let Text {
      text,
      font_id,
      font_size,
    } = text;
    Self {
      text,
      font_id,
      scale: font_size.into(),
      extra: (),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::*;

  use futures::executor::block_on;

  fn uninit_frame<'a>() -> Canvas {
    unsafe {
      let v = std::mem::MaybeUninit::uninit();
      v.assume_init()
    }
  }

  #[test]
  fn save_guard() {
    let mut layer = Rendering2DLayer::new();
    {
      let mut paint = layer.save();
      let t = Transform::row_major(1., 1., 1., 1., 1., 1.);
      paint.set_transform(t.clone());
      assert_eq!(&t, paint.get_transform());
      {
        let mut p2 = paint.save();
        let t2 = Transform::row_major(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.get_transform());
      }
      assert_eq!(&t, paint.get_transform());
    }
    assert_eq!(
      &Transform::row_major(1., 0., 0., 1., 0., 0.),
      layer.get_transform()
    );
  }

  #[test]
  fn buffer() {
    let mut layer = Rendering2DLayer::new();
    let mut frame = uninit_frame();
    let mut builder = Path::builder();
    builder.add_rectangle(
      &euclid::Rect::from_size((100., 100.).into()),
      Winding::Positive,
    );
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);
    let buffer = layer.finish(&mut frame).unwrap();

    assert!(!buffer.geometry.vertices.is_empty());
    assert_eq!(buffer.attrs.len(), 1);

    std::mem::forget(frame);
  }

  #[test]
  fn path_merge() {
    let mut layer = Rendering2DLayer::new();

    let mut frame = uninit_frame();

    let sample_path = Path::builder().build();
    // The stroke path both style and line width same should be merge.
    layer.stroke_path(sample_path.clone());
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut frame).unwrap().attrs.len(), 1);

    // Different line width with same color pen can be merged.
    layer.set_line_width(2.);
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut frame).unwrap().attrs.len(), 1);

    // Different color can't be merged.
    layer.set_style(FillStyle::Color(const_color::YELLOW.into()));
    layer.fill_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut frame).unwrap().attrs.len(), 2);

    // Different type style can't be merged
    layer.set_style(FillStyle::Image);
    layer.fill_path(sample_path.clone());
    layer.stroke_path(sample_path);
    assert_eq!(layer.clone().finish(&mut frame).unwrap().attrs.len(), 4);

    std::mem::forget(frame);
  }

  #[test]
  fn bounding_base() {
    let mut frame = uninit_frame();

    let layer = Rendering2DLayer::new();
    let mut path = Path::builder();
    path.add_rectangle(&lyon::geom::rect(100., 100., 50., 50.), Winding::Positive);
    let path = path.build();

    // color bounding min always zero.
    let mut l1 = layer.clone();
    l1.stroke_path(path.clone());
    assert_eq!(
      l1.finish(&mut frame).unwrap().attrs[0]
        .bounding_rect_for_style
        .min(),
      Point::new(0., 0.)
    );

    let mut l2 = layer;
    l2.set_style(FillStyle::Image);
    l2.stroke_path(path.clone());
    assert_eq!(
      l2.finish(&mut frame).unwrap().attrs[0]
        .bounding_rect_for_style
        .min(),
      Point::new(100., 100.)
    );

    std::mem::forget(frame);
  }

  #[test]
  #[ignore = "gpu need"]
  fn fill_text_hello() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    let font = canvas
      .add_font("DejaVuSans", include_bytes!("../fonts/DejaVuSans.ttf"))
      .unwrap();

    let mut layer = canvas.new_2d_layer();
    layer.set_font(font);
    layer.fill_text(Point::zero(), "Nice to meet you!", None);
    canvas.compose_2d_layer(layer);
    canvas.submit();

    unit_test::assert_canvas_eq!(canvas, "./test_imgs/text_hello.png");
  }
}

#[test]
#[ignore = "gpu need"]
fn fill_text_complex() {
  unimplemented!();
}

#[test]
#[ignore = "gpu need"]
fn fill_text_complex_single_style() {
  unimplemented!();
}

#[test]
#[ignore = "gpu need"]
fn update_texture_on_processing() {
  unimplemented!();
}
