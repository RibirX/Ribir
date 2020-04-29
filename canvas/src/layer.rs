pub use lyon::{
  math::{Point, Rect, Size, Transform},
  path::{builder::PathBuilder, Path},
};
use lyon::{path::Winding, tessellation::*};
pub use palette::{named, Srgba};
use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut},
};

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
  stroke_pen: STROKE_STYLE,
  fill_brush: FILL_STYLE,
};

pub struct Rendering2DLayer {
  state_stack: Vec<State>,
  commands: Vec<RenderCommand>,
}

impl Rendering2DLayer {
  pub(crate) fn new() -> Self {
    Self {
      state_stack: vec![],
      commands: vec![],
    }
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
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
      .map(|s| &s.stroke_pen)
      .unwrap_or(&STROKE_STYLE)
  }
  /// Returns the color, gradient, or pattern used for fill. Only `Color`
  /// support now.
  pub fn get_fill_style(&self) -> &FillStyle {
    self
      .state_stack
      .last()
      .map(|s| &s.fill_brush)
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
        stroke_pen: STROKE_STYLE,
        fill_brush: FILL_STYLE,
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

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, mut path: Path) {
    path = self.transform_path(path);
    self.commands.push(self.ctor_fill_command(path))
  }

  /// All drawing of this layer has finished, and convert the layer to an
  /// intermediate render buffer data that will provide to render process and
  /// then commit to gpu.
  pub fn finish(self) -> LayerBuffer2D {
    let mut buffer = LayerBuffer2D {
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
          disjoint_attr: DisjointAttr::Fill {},
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
          disjoint_attr: DisjointAttr::Stroke {},
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

#[derive(Debug, Clone, PartialEq)]
pub struct LayerBufferAttr {
  rg: std::ops::Range<usize>,
  style: FillStyle,
  disjoint_attr: DisjointAttr,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DisjointAttr {
  Fill {},
  Stroke {},
}

/// Layer buffer is the result of a layer drawing finished.
#[derive(Debug, Clone)]
pub struct LayerBuffer2D {
  geometry: VertexBuffers<Point, u16>,
  attrs: Vec<LayerBufferAttr>,
}

impl LayerBuffer2D {
  /// Return whether two buffer can be merged safely.
  pub(crate) fn mergeable(&self, other: &LayerBuffer2D) -> bool {
    self.geometry.vertices.len() + other.geometry.vertices.len()
      <= u16::MAX as usize
      && self.geometry.indices.len() + other.geometry.indices.len()
        <= u16::MAX as usize
  }

  /// Merge an other buffer into self, caller should use `mergeable` to check if
  /// safe to merge.
  pub(crate) fn merge(&mut self, other: &LayerBuffer2D) {
    fn append<T>(to: &mut Vec<T>, from: &Vec<T>) {
      // Point, U16 and LayerBufferAttr are safe to memory copy.
      unsafe {
        let count = from.len();
        to.reserve(count);
        let len = to.len();
        std::ptr::copy_nonoverlapping(
          from.as_ptr() as *const T,
          to.as_mut_ptr().add(len),
          count,
        );
        to.set_len(len + count);
      }
    }

    let offset_vertex = self.geometry.vertices.len();
    let offset_index = self.geometry.indices.len();
    let offset_attr = self.attrs.len();
    append(&mut self.geometry.vertices, &other.geometry.vertices);
    append(&mut self.geometry.indices, &other.geometry.indices);
    append(&mut self.attrs, &other.attrs);
    self
      .geometry
      .indices
      .iter_mut()
      .skip(offset_index)
      .for_each(|index| *index += offset_vertex as u16);
    self.attrs.iter_mut().skip(offset_attr).for_each(|attr| {
      attr.rg.start += offset_index;
      attr.rg.end += offset_index;
    });
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
  Color(Srgba<u8>),
}

#[derive(Clone)]
struct State {
  transform: Transform,
  stroke_pen: FillStyle,
  fill_brush: FillStyle,
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
    assert_eq!(&INIT_TRANSFORM, layer.get_transform());
  }

  #[test]
  fn buffer() {
    let mut layer = Rendering2DLayer::new();
    let mut builder = Path::builder();
    builder
      .add_rectangle(&Rect::from_size((100., 100.).into()), Winding::Positive);
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);
    let buffer = layer.finish();
    assert_eq!(buffer.attrs.len(), 2);
    assert!(!buffer.geometry.vertices.is_empty());
  }

  #[test]
  fn merge_buffer() {
    fn draw_point() -> LayerBuffer2D {
      let mut layer = Rendering2DLayer::new();
      let mut builder = Path::builder();
      builder.begin((0., 0.).into());
      builder.line_to((1., 0.).into());
      builder.line_to((1., 1.).into());
      builder.end(true);
      let path = builder.build();
      layer.fill_path(path);
      layer.finish()
    }
    let mut buffer1 = draw_point();
    let buffer2 = draw_point();
    assert!(buffer1.mergeable(&buffer2));
    buffer1.merge(&buffer2);
    debug_assert_eq!(&buffer1.geometry.indices, &[1, 0, 2, 4, 3, 5]);
    debug_assert_eq!(
      &buffer1.attrs,
      &[
        LayerBufferAttr {
          rg: 0..3,
          style: FILL_STYLE,
          disjoint_attr: DisjointAttr::Fill {}
        },
        LayerBufferAttr {
          rg: 3..6,
          style: FILL_STYLE,
          disjoint_attr: DisjointAttr::Fill {}
        }
      ]
    );
  }
}
