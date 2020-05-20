use crate::{
  text::{Section, Text, TextBrush},
  FrameImpl, Point, Rect, Size, Transform,
};
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

const DEFAULT_STATE: State = State {
  transform: Transform::row_major(1., 0., 0., 1., 0., 0.),
  stroke_pen: StrokePen {
    style: FillStyle::Color(Color {
      color: const_color::BLACK,
      alpha: u8::MAX,
    }),
    line_width: 1.,
  },
  fill_brush: Brush {
    style: FillStyle::Color(Color {
      color: const_color::WHITE,
      alpha: u8::MAX,
    }),
  },
};

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
      state_stack: vec![DEFAULT_STATE],
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

  /// Returns the color, gradient, or pattern used for strokes. Only `Color`
  /// support now.
  #[inline]
  pub fn get_stroke_pen_style(&self) -> &FillStyle {
    &self.current_state().stroke_pen.style
  }

  /// Change the style of pen that used to stroke path.
  #[inline]
  pub fn set_stroke_pen_style(&mut self, pen_style: FillStyle) -> &mut Self {
    self.current_state_mut().stroke_pen.style = pen_style;
    self
  }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn get_line_width(&self) -> f32 {
    self.current_state().stroke_pen.line_width
  }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().stroke_pen.line_width = line_width;
    self
  }

  /// Returns the color, gradient, or pattern used for fill. Only `Color`
  /// support now.
  #[inline]
  pub fn get_brush_style(&self) -> &FillStyle {
    &self.current_state().fill_brush.style
  }

  /// Change the style of brush that used to fill path.
  #[inline]
  pub fn set_brush_style(&mut self, pen_style: FillStyle) -> &mut Self {
    self.current_state_mut().fill_brush.style = pen_style;
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
    let path = Command {
      info: CommandInfo::Path(path),
      transform: state.transform,
      cmd_type: CommandType::Stroke(state.stroke_pen.clone()),
    };
    self.commands.push(path);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let state = self.current_state();
    let path = Command {
      info: CommandInfo::Path(path),
      transform: state.transform,
      cmd_type: CommandType::Fill(state.fill_brush.clone()),
    };
    self.commands.push(path);
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
  pub fn finish<S, T>(self, frame: &mut FrameImpl<S, T>) -> RenderCommand
  where
    S: crate::canvas::surface::Surface,
    T: std::borrow::Borrow<wgpu::TextureView>,
  {
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    let mut geometry = VertexBuffers::new();
    let mut attrs: Vec<RenderAttr> = vec![];

    self.commands.into_iter().for_each(|cmd| {
      let bounding_rect_for_style = cmd.bounding_rect_for_style();
      let Command {
        transform,
        info,
        cmd_type,
      } = cmd;

      let count = match info {
        CommandInfo::Path(path) => Self::tessellate_path(
          &mut geometry,
          path,
          &cmd_type,
          &mut fill_tess,
          &mut stroke_tess,
        ),
        CommandInfo::Text(sections) => {
          sections.into_iter().for_each(|section| {
            frame.canvas_mut().text_brush.queue(section);
          });

          let ptr = frame as *mut FrameImpl<S, T>;
          let brush = &mut frame.canvas_mut().text_brush;
          // Safe introduce:
          // reference circle, but canvas will not modify brush.
          let quad_vertices = unsafe { brush.process_queued(&mut *ptr) };
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
            let px_coords = rect_corners(&v.pixel_coords);
            let tex_coords = rect_corners(&v.tex_coords);
            geometry.vertices.push(Vertex {
              pixel_coords: px_coords[0],
              texture_coords: tex_coords[0],
            });
            geometry.vertices.push(Vertex {
              pixel_coords: px_coords[1],
              texture_coords: tex_coords[1],
            });
            geometry.vertices.push(Vertex {
              pixel_coords: px_coords[2],
              texture_coords: tex_coords[2],
            });
            geometry.vertices.push(Vertex {
              pixel_coords: px_coords[3],
              texture_coords: tex_coords[3],
            });

            let offset = geometry.indices.len();
            let tl = 0;
            let tr = 1 + offset as u32;
            let bl = 2 + offset as u32;
            let br = 3 + offset as u32;
            geometry.indices.push(tl);
            geometry.indices.push(tr);
            geometry.indices.push(bl);
            geometry.indices.push(bl);
            geometry.indices.push(tr);
            geometry.indices.push(br);
          });

          Count {
            vertices: quad_vertices.len() as u32 * 4,
            indices: quad_vertices.len() as u32 * 6,
          }
        }
      };

      let style = cmd_type.style();

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

  fn tessellate_path(
    mut buffer: &mut VertexBuffers<Vertex, u32>,
    path: Path,
    cmd_type: &CommandType,
    fill_tess: &mut FillTessellator,
    stroke_tess: &mut StrokeTessellator,
  ) -> Count {
    match cmd_type {
      CommandType::Fill(_) => fill_tess
        .tessellate_path(
          &path,
          &FillOptions::tolerance(TOLERANCE),
          &mut BuffersBuilder::new(&mut buffer, Vertex::from_fill_vertex),
        )
        .unwrap(),
      CommandType::Stroke(pen) => stroke_tess
        .tessellate_path(
          &path,
          &StrokeOptions::tolerance(TOLERANCE).with_line_width(pen.line_width),
          &mut BuffersBuilder::new(&mut buffer, Vertex::from_stroke_vertex),
        )
        .unwrap(),
    }
  }

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
#[derive(Clone, Debug)]
struct State {
  transform: Transform,
  stroke_pen: StrokePen,
  fill_brush: Brush,
}

#[derive(Debug, Clone, PartialEq)]
enum CommandType {
  Fill(Brush),
  Stroke(StrokePen),
}

impl CommandType {
  fn style(&self) -> &FillStyle {
    match self {
      CommandType::Stroke(pen) => &pen.style,
      CommandType::Fill(brush) => &brush.style,
    }
  }
}

#[derive(Debug, Clone)]
enum CommandInfo<'a> {
  Path(Path),
  Text(Vec<Section<'a>>),
}

#[derive(Debug, Clone)]
struct Command<'a> {
  info: CommandInfo<'a>,
  transform: Transform,
  cmd_type: CommandType,
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

#[cfg(test)]
mod test {
  use super::*;
  use crate::canvas::{surface::PhysicSurface, CanvasFrame};

  fn uninit_frame<'a>() -> CanvasFrame<'a, PhysicSurface> {
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
    assert_eq!(buffer.attrs.len(), 2);

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
}
