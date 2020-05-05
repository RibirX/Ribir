pub use lyon::{
  math::{Point, Rect, Size, Transform},
  path::{builder::PathBuilder, Path, Winding},
  tessellation::*,
};
pub use palette::{named as const_color, Srgba};
use std::{
  cmp::PartialEq,
  ops::{Deref, DerefMut, Range},
};

const tolerance: f32 = 0.02;
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
      transform: state.transform.clone(),
      cmd_type: PathCommandType::Stroke(state.stroke_pen.clone()),
    };
    self.add_path(path);
  }

  /// Use current brush fill the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let state = self.current_state();
    let path = PathCommand {
      path,
      transform: state.transform.clone(),
      cmd_type: PathCommandType::Fill(state.fill_brush.clone()),
    };
    self.add_path(path);
  }

  /// All drawing of this layer has finished, and convert the layer to an
  /// intermediate render buffer data that will provide to render process and
  /// then commit to gpu.
  pub fn finish(self) -> Vec<RenderCommand> {
    let mut buffer = vec![];
    let mut stroke_tess = StrokeTessellator::new();
    let mut fill_tess = FillTessellator::new();

    let mut last_cmd: Option<RenderCommand> = None;

    self.commands.into_iter().for_each(|cmd| {
      // If coming a different render command, crete a new render command.
      let PathCommand {
        transform,
        path,
        cmd_type,
      } = cmd;
      let color_cmd = matches!(cmd_type.style(), FillStyle::Color(_));
      if matches!(last_cmd, Some(RenderCommand::PureColor { .. })) != color_cmd
      {
        Self::push_cmd(&mut buffer, last_cmd.take());
      }
      if last_cmd.is_none() {
        let new_cmd = if color_cmd {
          RenderCommand::PureColor {
            geometry: VertexBuffers::new(),
            attrs: vec![],
          }
        } else {
          RenderCommand::Texture {
            geometry: VertexBuffers::new(),
            attrs: vec![],
          }
        };
        last_cmd = Some(new_cmd);
      }

      macro path_tess(
        $render_cmd_type: ident,
        $attr_type: ident,
        $attr_init: expr
      ) {
        if let Some(RenderCommand::$render_cmd_type { geometry, attrs }) =
          &mut last_cmd
        {
          let (rg, line_width) = Self::tessellate_path(
            geometry,
            path,
            &cmd_type,
            &mut fill_tess,
            &mut stroke_tess,
          );
          let rg_attr = RangeAttr {
            transform,
            line_width,
            rg,
          };
          attrs.push($attr_type::new(rg_attr, $attr_init));
        } else {
          unreachable!();
        }
      }

      match cmd_type.style() {
        FillStyle::Color(c) => {
          path_tess!(PureColor, ColorBufferAttr, c.clone());
        }
        FillStyle::Image => {
          path_tess!(Texture, TextureBufferAttr, TextureStyle::Image);
        }
        FillStyle::Gradient => {
          path_tess!(Texture, TextureBufferAttr, TextureStyle::Gradient);
        }
      }
    });

    Self::push_cmd(&mut buffer, last_cmd);
    buffer
  }

  fn push_cmd(buffer: &mut Vec<RenderCommand>, cmd: Option<RenderCommand>) {
    fn command_shrink<T>(
      geometry: &mut VertexBuffers<Point, u16>,
      attrs: &mut Vec<T>,
    ) {
      geometry.vertices.shrink_to_fit();
      geometry.indices.shrink_to_fit();
      attrs.shrink_to_fit()
    }
    if let Some(mut cmd) = cmd {
      match &mut cmd {
        RenderCommand::PureColor {
          ref mut geometry,
          ref mut attrs,
        } => {
          command_shrink(geometry, attrs);
        }
        RenderCommand::Texture {
          ref mut geometry,
          ref mut attrs,
        } => {
          command_shrink(geometry, attrs);
        }
      };
      buffer.push(cmd);
    }
  }

  fn tessellate_path(
    mut buffer: &mut VertexBuffers<Point, u16>,
    path: Path,
    cmd_type: &PathCommandType,
    fill_tess: &mut FillTessellator,
    stroke_tess: &mut StrokeTessellator,
  ) -> (Range<usize>, f32) {
    let start = buffer.indices.len();
    let line_width = match cmd_type {
      PathCommandType::Fill(_) => {
        fill_tess
          .tessellate_path(
            &path,
            &FillOptions::tolerance(tolerance),
            &mut BuffersBuilder::new(
              &mut buffer,
              |pos: Point, _: FillAttributes| pos,
            ),
          )
          .unwrap();

        1.
      }
      PathCommandType::Stroke(pen) => {
        stroke_tess
          .tessellate_path(
            &path,
            &StrokeOptions::tolerance(tolerance).dont_apply_line_width(),
            &mut BuffersBuilder::new(
              &mut buffer,
              |pos: Point, _: StrokeAttributes| pos,
            ),
          )
          .unwrap();
        pen.line_width
      }
    };
    (start..buffer.indices.len(), line_width)
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
        if c1.is_some() && c1 == c2 && lw1 == lw2 {
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
  pub(crate) line_width: f32,
  pub(crate) transform: Transform,
}

#[derive(Debug, Clone)]
pub struct ColorBufferAttr {
  pub(crate) rg_attr: RangeAttr,
  pub(crate) color: Color,
}

impl ColorBufferAttr {
  #[inline]
  fn new(rg_attr: RangeAttr, color: Color) -> Self { Self { rg_attr, color } }
}

#[derive(Debug, Clone)]
pub(crate) enum TextureStyle {
  Image,
  Gradient,
}

#[derive(Debug, Clone)]
pub struct TextureBufferAttr {
  pub(crate) rg_attr: RangeAttr,
  pub(crate) texture: TextureStyle,
}

impl TextureBufferAttr {
  #[inline]
  fn new(rg_attr: RangeAttr, texture: TextureStyle) -> Self {
    Self { rg_attr, texture }
  }
}

trait Attr {
  fn range_attr(&mut self) -> &mut RangeAttr;
}

impl Attr for ColorBufferAttr {
  #[inline]
  fn range_attr(&mut self) -> &mut RangeAttr { &mut self.rg_attr }
}

impl Attr for TextureBufferAttr {
  #[inline]
  fn range_attr(&mut self) -> &mut RangeAttr { &mut self.rg_attr }
}

#[derive(Debug, Clone)]
pub enum RenderCommand {
  PureColor {
    geometry: VertexBuffers<Point, u16>,
    attrs: Vec<ColorBufferAttr>,
  },
  Texture {
    geometry: VertexBuffers<Point, u16>,
    attrs: Vec<TextureBufferAttr>,
  },
}

impl RenderCommand {
  pub(crate) fn vertices(&self) -> &Vec<Point> {
    match self {
      RenderCommand::PureColor { geometry, .. } => &geometry.vertices,
      RenderCommand::Texture { geometry, .. } => &geometry.vertices,
    }
  }

  pub(crate) fn indices(&self) -> &Vec<u16> {
    match self {
      RenderCommand::PureColor { geometry, .. } => &geometry.indices,
      RenderCommand::Texture { geometry, .. } => &geometry.indices,
    }
  }

  /// Merge an other render command, return true if merge successful other
  /// false.
  pub(crate) fn merge(&mut self, other: &Self) -> bool {
    match self {
      RenderCommand::PureColor { geometry, attrs } => {
        if let RenderCommand::PureColor {
          geometry: other_geometry,
          attrs: other_attrs,
        } = other
        {
          Self::merge_data(geometry, attrs, other_geometry, other_attrs);
          true
        } else {
          false
        }
      }
      RenderCommand::Texture { geometry, attrs } => {
        if let RenderCommand::Texture {
          geometry: other_geometry,
          attrs: other_attrs,
        } = other
        {
          Self::merge_data(geometry, attrs, other_geometry, other_attrs);
          true
        } else {
          false
        }
      }
      _ => false,
    }
  }

  fn merge_data<T: Attr>(
    geometry: &mut VertexBuffers<Point, u16>,
    attrs: &mut Vec<T>,
    other_geometry: &VertexBuffers<Point, u16>,
    other_attrs: &Vec<T>,
  ) {
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

    let vertex_offset = geometry.vertices.len();
    let indices_offset = geometry.indices.len();
    let offset_attr = attrs.len();
    append(&mut geometry.vertices, &other_geometry.vertices);
    append(&mut geometry.indices, &other_geometry.indices);
    append(attrs, &other_attrs);

    geometry
      .indices
      .iter_mut()
      .skip(indices_offset)
      .for_each(|index| *index += vertex_offset as u16);
    attrs.iter_mut().skip(offset_attr).for_each(|attr| {
      let rg_attr = attr.range_attr();
      rg_attr.rg.start += indices_offset;
      rg_attr.rg.end += indices_offset;
    });
  }
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
    builder
      .add_rectangle(&Rect::from_size((100., 100.).into()), Winding::Positive);
    let path = builder.build();
    layer.stroke_path(path.clone());
    layer.fill_path(path);
    let buffer = layer.finish();
    assert_eq!(buffer.len(), 1);
    assert!(!buffer[0].vertices().is_empty());
    let attrs = match &buffer[0] {
      RenderCommand::PureColor { attrs, .. } => attrs.len(),
      RenderCommand::Texture { attrs, .. } => attrs.len(),
    };
    assert_eq!(attrs, 2);
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
    layer.stroke_path(sample_path.clone());
    assert_eq!(layer.commands.len(), 5);
  }

  #[test]
  fn merge_buffer() {
    fn draw() -> Vec<RenderCommand> {
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
    let cmd1 = &mut draw()[0];
    let cmd2 = &draw()[0];

    assert!(cmd1.merge(&cmd2));
    assert_eq!(cmd1.indices(), &[1, 0, 2, 4, 3, 5]);

    if let RenderCommand::PureColor { attrs, .. } = cmd1 {
      assert_eq!(attrs.len(), 2);
      assert_eq!(attrs[0].rg_attr.rg, 0..3);
      assert_eq!(attrs[1].rg_attr.rg, 3..6);
    } else {
      unreachable!();
    }
  }
}
