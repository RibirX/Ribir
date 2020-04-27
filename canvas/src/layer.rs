use lyon::tessellation::*;
use lyon::{
  math::{Point, Transform},
  path::Path,
};
use palette::{named, Srgba};
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, Debug)]
struct Vertex([f32; 2]);

const STROKE_STYLE: FillStyle = FillStyle::Color(Srgba {
  color: named::BLACK,
  alpha: u8::MAX,
});

const FILL_STYLE: FillStyle = FillStyle::Color(Srgba {
  color: named::WHITE,
  alpha: u8::MAX,
});

const INIT_TRANSFORM: Transform = Transform::row_major(1., 0., 0., 1., 0., 0.);

const DEFAULT_STATE: State = State {
  transform: INIT_TRANSFORM,
  stroke_style: STROKE_STYLE,
  fill_style: FILL_STYLE,
};

pub struct Rendering2DLayer {
  state_stack: Vec<State>,
  commands: Vec<RenderCommand>,
}

impl Rendering2DLayer {
  pub fn new() -> Self {
    Self {
      state_stack: vec![],
      commands: vec![],
    }
  }

  #[must_use]
  pub fn save(&mut self) -> LayerGuard {
    let new_state = if let Some(last) = self.state_stack.last() {
      last.clone()
    } else {
      DEFAULT_STATE.clone()
    };
    self.state_stack.push(new_state);
    LayerGuard(self)
  }

  /// Returns the color, gradient, or pattern used for strokes. Only `Color`
  /// support now.
  pub fn get_stroke_style(&self) -> &FillStyle {
    self
      .state_stack
      .last()
      .map(|s| &s.stroke_style)
      .unwrap_or(&STROKE_STYLE)
  }
  /// Returns the color, gradient, or pattern used for fill. Only `Color`
  /// support now.
  pub fn get_fill_style(&self) -> &FillStyle {
    self
      .state_stack
      .last()
      .map(|s| &s.fill_style)
      .unwrap_or(&FILL_STYLE)
  }

  /// Return the current transformation matrix being applied to the layer.
  pub fn get_transform(&self) -> &Transform {
    self
      .state_stack
      .last()
      .map(|s| &s.transform)
      .unwrap_or(&INIT_TRANSFORM)
  }

  /// Resets (overrides) the current transformation to the identity matrix, and
  /// then invokes a transformation described by the arguments of this method.
  /// This lets you scale, rotate, translate (move), and skew the context.
  pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
    if let Some(state) = self.state_stack.last_mut() {
      state.transform = transform;
    } else {
      self.state_stack.push(State {
        stroke_style: STROKE_STYLE,
        fill_style: FILL_STYLE,
        transform,
      });
    }

    self
  }

  /// Renders the specified path by using the current pen.
  pub fn stroke_path(&mut self, mut path: Path) {
    path = self.transform_path(path);
    self.commands.push(self.ctor_stroke_command(path))
  }

  /// Fills the interior of the `path`.
  pub fn fill_path(&mut self, mut path: Path) {
    path = self.transform_path(path);
    self.commands.push(self.ctor_fill_command(path))
  }

  pub fn finish(self) -> LayerBuffer {
    let mut buffer = LayerBuffer {
      geometry: VertexBuffers::new(),
      attrs: vec![],
    };
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    self.commands.into_iter().for_each(|cmd| match cmd {
      RenderCommand::Fill { path } => {
        let start = buffer.geometry.indices.len();
        fill_tess
          .tessellate_path(
            &path.path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(
              &mut buffer.geometry,
              |pos: Point, _: FillAttributes| pos,
            ),
          )
          .unwrap();
        buffer.attrs.push(LayerBufferAttr {
          rg: start..buffer.geometry.indices.len(),
          style: path.style,
        })
      }
      RenderCommand::Stroke { path } => {
        let start = buffer.geometry.indices.len();
        stroke_tess
          .tessellate_path(
            &path.path,
            &StrokeOptions::default().dont_apply_line_width(),
            &mut BuffersBuilder::new(
              &mut buffer.geometry,
              |pos: Point, _: StrokeAttributes| pos,
            ),
          )
          .unwrap();

        buffer.attrs.push(LayerBufferAttr {
          rg: start..buffer.geometry.indices.len(),
          style: path.style,
        })
      }
    });

    buffer
  }

  fn ctor_fill_command(&self, path: Path) -> RenderCommand {
    RenderCommand::Fill {
      path: PathCommand {
        path,
        style: self.get_fill_style().clone(),
      },
    }
  }

  fn ctor_stroke_command(&self, path: Path) -> RenderCommand {
    RenderCommand::Stroke {
      path: PathCommand {
        path,
        style: self.get_stroke_style().clone(),
      },
    }
  }

  fn transform_path(&self, mut path: Path) -> Path {
    if let Some(state) = self.state_stack.last() {
      path = path.transformed(&state.transform)
    }
    path
  }
}

pub struct LayerBufferAttr {
  rg: std::ops::Range<usize>,
  style: FillStyle,
}

pub struct LayerBuffer {
  geometry: VertexBuffers<Point, u16>,
  attrs: Vec<LayerBufferAttr>,
}

#[derive(Clone)]
pub enum FillStyle {
  Color(Srgba<u8>),
}

#[derive(Clone)]
struct State {
  transform: Transform,
  stroke_style: FillStyle,
  fill_style: FillStyle,
}

impl State {}

enum RenderCommand {
  Fill { path: PathCommand },
  Stroke { path: PathCommand },
}

struct PathCommand {
  path: Path,
  style: FillStyle,
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
  assert_eq!(&INIT_TRANSFORM, layer.get_transform());
}
