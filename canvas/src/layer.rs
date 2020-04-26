use lyon::tessellation::*;
use lyon::{
  math::{Point, Transform},
  path::Path,
};
use palette::{named, Srgba};

#[derive(Copy, Clone, Debug)]
struct Vertex([f32; 2]);

pub struct Painter {}

pub struct Rendering2DLayer {
  state_stack: Vec<State>,
  commands: Vec<RenderCommand>,
}

impl Rendering2DLayer {
  pub fn new() -> Self {
    Self {
      state_stack: vec![State::new()],
      commands: vec![],
    }
  }

  /// Renders the specified path by using the current pen.
  pub fn stroke_path(&mut self, path: Path) {
    let mut tessellator = StrokeTessellator::new();
    let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
    tessellator
      .tessellate_path(
        &path.transformed(&self.current_state().transform),
        &StrokeOptions::default().dont_apply_line_width(),
        &mut BuffersBuilder::new(
          &mut geometry,
          |pos: Point, _: StrokeAttributes| pos,
        ),
      )
      .unwrap();
    self.commands.push(self.ctor_stroke_command(geometry))
  }

  /// Fills the interior of the `path`.
  pub fn fill_path(&mut self, path: Path) {
    let mut tessellator = FillTessellator::new();
    let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
    tessellator
      .tessellate_path(
        &path.transformed(&self.current_state().transform),
        &FillOptions::default(),
        &mut BuffersBuilder::new(
          &mut geometry,
          |pos: Point, _: FillAttributes| pos,
        ),
      )
      .unwrap();
    self.commands.push(self.ctor_fill_command(geometry))
  }

  fn ctor_fill_command(
    &self,
    geometry: VertexBuffers<Point, u16>,
  ) -> RenderCommand {
    let mut command = RenderCommand {
      geometry,
      tess_type: TessellationType::Fill,
      style: self.current_state().fill_style.clone(),
    };
    command.shrink();
    command
  }

  fn ctor_stroke_command(
    &self,
    geometry: VertexBuffers<Point, u16>,
  ) -> RenderCommand {
    let mut command = RenderCommand {
      geometry,
      tess_type: TessellationType::Stroke,
      style: self.current_state().stroke_style.clone(),
    };
    command.shrink();
    command
  }

  fn current_state(&self) -> &State {
    self
      .state_stack
      .last()
      .expect("Should always have one state in stack")
  }
}

#[derive(Clone)]
enum FillStyle {
  Color(Srgba<u8>),
}

struct State {
  transform: Transform,
  stroke_style: FillStyle,
  fill_style: FillStyle,
}

impl State {
  const fn new() -> Self {
    Self {
      transform: Transform::row_major(1., 0., 0., 1., 0., 0.),
      stroke_style: FillStyle::Color(Srgba {
        color: named::BLACK,
        alpha: u8::MAX,
      }),
      fill_style: FillStyle::Color(Srgba {
        color: named::WHITE,
        alpha: u8::MAX,
      }),
    }
  }
}
enum TessellationType {
  Fill,
  Stroke,
}
struct RenderCommand {
  geometry: VertexBuffers<Point, u16>,
  tess_type: TessellationType,
  style: FillStyle,
}

impl RenderCommand {
  fn shrink(&mut self) {
    self.geometry.vertices.shrink_to_fit();
    self.geometry.indices.shrink_to_fit();
  }
}
