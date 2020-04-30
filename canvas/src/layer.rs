pub use lyon::{
  math::{Point, Rect, Size, Transform},
  path::{builder::PathBuilder, Path},
};
use lyon::{path::Winding, tessellation::*};
use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut, Range},
};

const DEFAULT_STATE: State = State {
  transform: Transform::row_major(1., 0., 0., 1., 0., 0.),
  stroke_pen: StrokePen {
    style: FillStyle::Color(wgpu::Color::BLACK),
    line_width: 1.,
  },
  fill_brush: Brush {
    style: FillStyle::Color(wgpu::Color::WHITE),
  },
};

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
  /// Returns the color, gradient, or pattern used for fill. Only `Color`
  /// support now.
  #[inline]
  pub fn get_brush_style(&self) -> &FillStyle {
    &self.current_state().fill_brush.style
  }

  /// Return the current transformation matrix being applied to the layer.
  #[inline]
  pub fn get_transform(&self) -> &Transform { &self.current_state().transform }

  /// Resets (overrides) the current transformation to the identity matrix, and
  /// then invokes a transformation described by the arguments of this method.
  /// This lets you scale, rotate, translate (move), and skew the context.
  pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
    let mut state = self.state_stack.last_mut().expect("must have one level");
    state.transform = transform;

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

    self.commands.into_iter().for_each(|cmd| {
      let PathCommand {
        transform,
        path,
        cmd_type,
      } = cmd;
      let start = buffer.geometry.indices.len();
      let (style, line_width) = match cmd_type {
        CommandType::Fill(brush) => {
          fill_tess
            .tessellate_path(
              &path,
              &FillOptions::default(),
              &mut BuffersBuilder::new(
                &mut buffer.geometry,
                |pos: Point, _: FillAttributes| pos,
              ),
            )
            .unwrap();

          (brush.style, 1.)
        }
        CommandType::Stroke(pen) => {
          stroke_tess
            .tessellate_path(
              &path,
              &StrokeOptions::default().dont_apply_line_width(),
              &mut BuffersBuilder::new(
                &mut buffer.geometry,
                |pos: Point, _: StrokeAttributes| pos,
              ),
            )
            .unwrap();
          (pen.style, pen.line_width)
        }
      };
      buffer.attrs.push(LayerBufferAttr {
        rg: start..buffer.geometry.indices.len(),
        transform,
        style,
        line_width,
      });
    });

    buffer
  }

  fn ctor_fill_command(&self, path: Path) -> PathCommand {
    let state = self.current_state();
    PathCommand {
      path,
      transform: state.transform.clone(),
      cmd_type: CommandType::Fill(state.fill_brush.clone()),
    }
  }

  fn ctor_stroke_command(&self, path: Path) -> PathCommand {
    let state = self.current_state();
    PathCommand {
      path,
      transform: state.transform.clone(),
      cmd_type: CommandType::Stroke(state.stroke_pen.clone()),
    }
  }

  fn transform_path(&self, mut path: Path) -> Path {
    if let Some(state) = self.state_stack.last() {
      path = path.transformed(&state.transform)
    }
    path
  }

  fn current_state(&self) -> &State {
    self
      .state_stack
      .last()
      .expect("Must have one state in stack!")
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerBufferAttr {
  rg: Range<usize>,
  style: FillStyle,
  line_width: f32,
  transform: Transform,
}

/// Layer buffer is the result of a layer drawing finished.
#[derive(Debug, Clone)]
pub struct LayerBuffer2D {
  geometry: VertexBuffers<Point, u16>,
  attrs: Vec<LayerBufferAttr>,
}

impl LayerBuffer2D {
  pub fn iter_same_style(&self) -> SameTypeIter {
    SameTypeIter {
      buffer: self,
      idx: 0,
    }
  }
}

/// The iterator for `LayerBuffer2D` that iter the same style attrs range.
pub struct SameTypeIter<'a> {
  buffer: &'a LayerBuffer2D,
  idx: usize,
}

impl<'a> Iterator for SameTypeIter<'a> {
  type Item = Range<usize>;
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(attr) = self.buffer.attrs.get(self.idx) {
      let pure_color = matches!(attr.style, FillStyle::Color(_));
      let start = self.idx;
      // Find the first different style attr.
      let idx = self.buffer.attrs[start..]
        .iter()
        .position(|attr| {
          matches!(attr.style, FillStyle::Color(_)) != pure_color
        })
        .map_or(self.buffer.attrs.len(), |offset| start + offset);
      self.idx = idx;
      Some(start..idx)
    } else {
      None
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
  Color(wgpu::Color),
  Image,    // todo
  Gradient, // todo,
}

#[derive(Clone)]
struct StrokePen {
  style: FillStyle,
  line_width: f32,
}

#[derive(Clone)]
struct Brush {
  style: FillStyle,
}
#[derive(Clone)]
struct State {
  transform: Transform,
  stroke_pen: StrokePen,
  fill_brush: Brush,
}

enum CommandType {
  Fill(Brush),
  Stroke(StrokePen),
}
struct PathCommand {
  path: Path,
  transform: Transform,
  cmd_type: CommandType,
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
  fn buffer_iter() {
    fn creat_attr(rg: Range<usize>, style: FillStyle) -> LayerBufferAttr {
      LayerBufferAttr {
        rg,
        style,
        line_width: 1.,
        transform: Transform::row_major(0., 0., 0., 0., 0., 0.),
      }
    }
    let buffer = LayerBuffer2D {
      geometry: VertexBuffers::new(),
      //In real world, attrs should always match geometry.
      attrs: vec![
        creat_attr(0..10, FillStyle::Color(wgpu::Color::BLACK)),
        creat_attr(10..20, FillStyle::Color(wgpu::Color::WHITE)),
        creat_attr(20..30, FillStyle::Image),
        creat_attr(30..40, FillStyle::Gradient),
        creat_attr(40..50, FillStyle::Color(wgpu::Color::WHITE)),
      ],
    };

    let mut iter = buffer.iter_same_style();
    assert_eq!(iter.next(), Some(0..2));
    assert_eq!(iter.next(), Some(2..4));
    assert_eq!(iter.next(), Some(4..5));
    assert_eq!(iter.next(), None);
  }
}
