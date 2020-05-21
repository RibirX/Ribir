use crate::{text::Section, Canvas, Point, Rect, Size, Transform};
pub use glyph_brush::{FontId, HorizontalAlign, Layout, VerticalAlign};
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
    let state = self.current_state();
    let cmd = self.command_from_path(path, true);
    self.commands.push(cmd);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let state = self.current_state();
    let cmd = self.command_from_path(path, false);
    self.commands.push(cmd);
  }

  pub fn fill_text_with_desc(&mut self) {
    unimplemented!();
  }

  /// Fill `text` from left to right, start at `left_top`.
  /// Partially hitting the `max_width` will end the draw.
  /// Use `font` and `font_size` to specify the font and font size.
  /// Use [`fill_text_with_desc`](Rendering2DLayer::fill_text_with_desc) method
  /// to fill complex text.
  pub fn fill_text(
    &mut self,
    left_top: Point,
    text: &'a str,
    max_width: Option<f32>,
  ) {
    let state = self.current_state();
    let mut sec = Section::new().with_screen_position(left_top).add_text(
      glyph_brush::Text::default()
        .with_text(text)
        // fixme: text should have style
        //.with_extra(state.style)
        .with_font_id(state.font)
        .with_scale(state.font_size),
    );
    if let Some(max_width) = max_width {
      sec = sec.with_bounds((max_width, f32::MAX))
    }
    if let Some(cmd) = self.commands.last_mut() {
      if let CommandInfo::Text(ref mut texts) = cmd.info {
        if state.transform == cmd.transform {
          texts.push(sec);
          return;
        }
      }
    }

    let cmd = Command {
      info: CommandInfo::Text(vec![sec]),
      transform: state.transform,
    };
    self.commands.push(cmd);
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

  /// All drawing of this layer has finished, and convert the layer to an
  /// intermediate render buffer data that will provide to render process and
  /// then commit to gpu.
  pub fn finish<S>(self, canvas: &mut Canvas<S>) -> RenderCommand
  where
    S: crate::canvas::surface::Surface,
  {
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    let mut geometry = VertexBuffers::new();
    let mut attrs: Vec<RenderAttr> = vec![];

    self.commands.into_iter().for_each(|cmd| {
      let bounding_rect_for_style = cmd.bounding_rect_for_style();
      let Command { transform, info } = cmd;

      let count = match info {
        CommandInfo::Path {
          path,
          style,
          stroke_line_width,
        } => {
          if let Some(line_width) = stroke_line_width {
            stroke_tess
              .tessellate_path(
                &path,
                &StrokeOptions::tolerance(TOLERANCE)
                  .with_line_width(line_width),
                &mut BuffersBuilder::new(
                  &mut geometry,
                  Vertex::from_stroke_vertex,
                ),
              )
              .unwrap()
          } else {
            fill_tess
              .tessellate_path(
                &path,
                &FillOptions::tolerance(TOLERANCE),
                &mut BuffersBuilder::new(
                  &mut geometry,
                  Vertex::from_fill_vertex,
                ),
              )
              .unwrap()
          }
        }
        CommandInfo::Text(sections) => {
          let quad_vertices = canvas.process_text_sections(sections);
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
            let VertexBuffers { vertices, indices } = &mut geometry;
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

          Count {
            vertices: quad_vertices.len() as u32 * 4,
            indices: quad_vertices.len() as u32 * 6,
          }
        }
      };

      if let Some(last) = attrs.last_mut() {
        if last.bounding_rect_for_style == bounding_rect_for_style
          && &last.style == style
          && last.transform == transform
        {
          last.count.vertices += count.vertices;
          last.count.indices += count.indices;
          return;
        }
      }

      attrs.push(RenderAttr {
        transform,
        bounding_rect_for_style,
        count,
        style: style.clone(),
      });
    });

    RenderCommand { geometry, attrs }
  }
}

/// Describe render the text as single line or break as multiple lines.
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct TextLayout {
  pub h_align: HorizontalAlign,
  pub v_align: VerticalAlign,
  pub wrap: LineWrap,
}

pub struct TextDesc<'a> {
  /// Box bounds, in pixels from top-left. Defaults to unbounded.
  pub bounds: Rect,
  pub layout: TextLayout,
  /// Text to render, rendered next to one another according the layout.
  pub text: Vec<Text<'a>>,
}

pub struct Text<'a> {
  /// Text to render
  pub text: &'a str,
  /// Text pixel size.
  pub font_size: f32,
  /// It must be a valid id of font, can query font id from
  /// [`Canvas::get_font_id_by_name`](Canvas::get_font_id_by_name) or across
  /// canvas to load custom font The default `FontId(0)` should always be
  pub font_id: FontId,
  /// Style to render text
  pub style: FillStyle,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderAttr {
  pub(crate) count: Count,
  pub(crate) transform: Transform,
  pub(crate) style: FillStyle,
  pub(crate) bounding_rect_for_style: Rect,
}

#[derive(Debug, Clone)]
pub struct Vertex {
  pub(crate) pixel_coords: Point,
  pub(crate) texture_coords: Point,
}

#[derive(Debug, Clone)]
pub struct RenderCommand {
  pub(crate) geometry: VertexBuffers<Vertex, u32>,
  pub(crate) attrs: Vec<RenderAttr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
  Color(Color),
  Image,    // todo
  Gradient, // todo,
}

#[derive(Clone, PartialEq, Debug)]
struct StrokePen {
  style: FillStyle,
  line_width: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Brush {
  style: FillStyle,
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

  fn command_from_path<'l>(
    &self,
    path: Path,
    stroke_or_fill: bool,
  ) -> Command<'l> {
    let state = self.current_state();
    let stroke_line_width = if stroke_or_fill {
      Some(self.current_state().line_width)
    } else {
      None
    };
    Command {
      info: CommandInfo::Path {
        path,
        style: state.style,
        stroke_line_width,
      },
      transform: state.transform,
    }
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
    stroke_line_width: Option<f32>,
  },
  Text(Vec<Section<'a>>),
}

#[derive(Debug, Clone)]
struct Command<'a> {
  info: CommandInfo<'a>,
  transform: Transform,
}

impl<'a> Command<'a> {
  fn bounding_rect_for_style(&self) -> Rect {
    if let FillStyle::Color(_) = self.cmd_type.style() {
      // Pure color just one pixel in texture, and always use repeat pattern, so
      // zero min is ok, no matter what really bounding it is.
      Rect::new(Point::new(0., 0.), Size::new(1., 1.))
    } else {
      if let CommandInfo::Path(ref path) = self.info {
        let rect = lyon::algorithms::aabb::bounding_rect(path.iter());
        Rect::from_untyped(&rect)
      } else {
        unimplemented!("text texture bounding not support now");
      }
    }
  }
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
    let buffer = layer.finish(&mut frame);

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
    assert_eq!(layer.clone().finish(&mut frame).attrs.len(), 1);

    // Different line width with same color pen can be merged.
    layer.set_line_width(2.);
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut frame).attrs.len(), 1);

    // Different color can't be merged.
    layer.set_brush_style(FillStyle::Color(const_color::YELLOW.into()));
    layer.fill_path(sample_path.clone());
    assert_eq!(layer.clone().finish(&mut frame).attrs.len(), 2);

    // Different type style can't be merged
    layer.set_brush_style(FillStyle::Image);
    layer.fill_path(sample_path.clone());
    layer.stroke_path(sample_path);
    assert_eq!(layer.clone().finish(&mut frame).attrs.len(), 4);

    std::mem::forget(frame);
  }

  #[test]
  fn bounding_base() {
    let mut frame = uninit_frame();

    let layer = Rendering2DLayer::new();
    let mut path = Path::builder();
    path.add_rectangle(
      &lyon::geom::rect(100., 100., 50., 50.),
      Winding::Positive,
    );
    let path = path.build();

    // color bounding min always zero.
    let mut l1 = layer.clone();
    l1.stroke_path(path.clone());
    assert_eq!(
      l1.finish(&mut frame).attrs[0].bounding_rect_for_style.min(),
      Point::new(0., 0.)
    );

    let mut l2 = layer;
    l2.set_stroke_pen_style(FillStyle::Image);
    l2.stroke_path(path.clone());
    assert_eq!(
      l2.finish(&mut frame).attrs[0].bounding_rect_for_style.min(),
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
