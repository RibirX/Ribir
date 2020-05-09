use crate::{Point, Transform};
pub use lyon::{
  path::{builder::PathBuilder, Path, Winding},
  tessellation::*,
};
pub use palette::{named as const_color, Srgba};
use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut, Range},
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
pub struct Rendering2DLayer {
  state_stack: Vec<State>,
  commands: Vec<PathCommand>,
}

impl Rendering2DLayer {
  pub(crate) fn new() -> Self {
    Self {
      state_stack: vec![DEFAULT_STATE],
      commands: vec![],
    }
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
  #[must_use]
  pub fn save(&mut self) -> LayerGuard {
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
    let path = PathCommand {
      path,
      transform: state.transform,
      cmd_type: PathCommandType::Stroke(state.stroke_pen.clone()),
    };
    self.add_path(path);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let state = self.current_state();
    let path = PathCommand {
      path,
      transform: state.transform,
      cmd_type: PathCommandType::Fill(state.fill_brush.clone()),
    };
    self.add_path(path);
  }

  /// All drawing of this layer has finished, and convert the layer to an
  /// intermediate render buffer data that will provide to render process and
  /// then commit to gpu.
  pub fn finish(self) -> RenderCommand {
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    let mut geometry = VertexBuffers::new();
    let mut attrs = vec![];

    self.commands.into_iter().for_each(|cmd| {
      // If coming a different render command, crete a new render command.
      let PathCommand {
        transform,
        path,
        cmd_type,
      } = cmd;

      let rg = Self::tessellate_path(
        &mut geometry,
        path,
        &cmd_type,
        &mut fill_tess,
        &mut stroke_tess,
      );
      let rg_attr = RangeAttr {
        transform,
        rg,
        style: cmd_type.style().clone(),
      };
      attrs.push(rg_attr);
    });

    RenderCommand { geometry, attrs }
  }

  fn tessellate_path(
    mut buffer: &mut VertexBuffers<Point, u32>,
    path: Path,
    cmd_type: &PathCommandType,
    fill_tess: &mut FillTessellator,
    stroke_tess: &mut StrokeTessellator,
  ) -> Range<usize> {
    let start = buffer.indices.len();
    match cmd_type {
      PathCommandType::Fill(_) => {
        fill_tess
          .tessellate_path(
            &path,
            &FillOptions::tolerance(TOLERANCE),
            &mut BuffersBuilder::new(&mut buffer, |vertex: FillVertex| {
              Point::from_untyped(vertex.position())
            }),
          )
          .unwrap();
      }
      PathCommandType::Stroke(pen) => {
        stroke_tess
          .tessellate_path(
            &path,
            &StrokeOptions::tolerance(TOLERANCE)
              .with_line_width(pen.line_width),
            &mut BuffersBuilder::new(&mut buffer, |vertex: StrokeVertex| {
              Point::from_untyped(vertex.position())
            }),
          )
          .unwrap();
      }
    };
    start..buffer.indices.len()
  }

  fn add_path(&mut self, path: PathCommand) {
    // Try to merge path
    if let Some(last) = self.commands.last_mut() {
      fn style_color(style: &FillStyle) -> Option<&Color> {
        if let FillStyle::Color(c) = style {
          Some(c)
        } else {
          None
        }
      }
      fn key_info(cmd_type: &PathCommandType) -> (Option<&Color>, f32) {
        match cmd_type {
          PathCommandType::Stroke(pen) => {
            (style_color(&pen.style), pen.line_width)
          }
          PathCommandType::Fill(brush) => (style_color(&brush.style), 1.),
        }
      }
      if last.transform == path.transform {
        let (c1, lw1) = key_info(&last.cmd_type);
        let (c2, lw2) = key_info(&path.cmd_type);
        if c1.is_some() && c1 == c2 && (lw1 - lw2).abs() < f32::EPSILON {
          let mut builder = Path::builder();
          builder.concatenate(&[last.path.as_slice(), path.path.as_slice()]);
          last.path = builder.build();
          return;
        }
      }
    }
    self.commands.push(path)
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
pub(crate) struct RangeAttr {
  pub(crate) rg: Range<usize>,
  pub(crate) transform: Transform,
  pub(crate) style: FillStyle,
}

#[derive(Debug, Clone)]
pub struct RenderCommand {
  pub(crate) geometry: VertexBuffers<Point, u32>,
  pub(crate) attrs: Vec<RangeAttr>,
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
#[derive(Clone)]
struct State {
  transform: Transform,
  stroke_pen: StrokePen,
  fill_brush: Brush,
}

#[derive(Debug, Clone, PartialEq)]
enum PathCommandType {
  Fill(Brush),
  Stroke(StrokePen),
}

impl PathCommandType {
  fn style(&self) -> &FillStyle {
    match self {
      PathCommandType::Stroke(pen) => &pen.style,
      PathCommandType::Fill(brush) => &brush.style,
    }
  }
}

struct PathCommand {
  path: Path,
  transform: Transform,
  cmd_type: PathCommandType,
}

/// An RAII implementation of a "scoped state" of the render layer. When this
/// structure is dropped (falls out of scope), changed state will auto restore.
/// The data can be accessed through this guard via its Deref and DerefMut
/// implementations.
pub struct LayerGuard<'a>(&'a mut Rendering2DLayer);

impl<'a> Drop for LayerGuard<'a> {
  #[inline]
  fn drop(&mut self) {
    debug_assert!(!self.0.state_stack.is_empty());
    self.0.state_stack.pop();
  }
}

impl<'a> Deref for LayerGuard<'a> {
  type Target = Rendering2DLayer;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a> DerefMut for LayerGuard<'a> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

#[cfg(test)]
mod test {
  use super::*;

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
    let mut builder = Path::builder();
    builder.add_rectangle(
      &euclid::Rect::from_size((100., 100.).into()),
      Winding::Positive,
    );
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);
    let buffer = layer.finish();

    assert!(!buffer.geometry.vertices.is_empty());
    assert_eq!(buffer.attrs.len(), 2);
  }

  #[test]
  fn path_merge() {
    let mut layer = Rendering2DLayer::new();

    let sample_path = Path::builder().build();
    // The stroke path both style and line width same should be merge.
    layer.stroke_path(sample_path.clone());
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.commands.len(), 1);

    // Different line width pen can't be merged.
    layer.set_line_width(2.);
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.commands.len(), 2);

    // Stroke and fill can't be merged.
    layer.set_brush_style(FillStyle::Color(const_color::YELLOW.into()));
    layer.fill_path(sample_path.clone());
    assert_eq!(layer.commands.len(), 3);

    // Different type style can't be merged
    layer.set_brush_style(FillStyle::Image);
    layer.fill_path(sample_path.clone());
    layer.stroke_path(sample_path);
    assert_eq!(layer.commands.len(), 5);
  }
}
