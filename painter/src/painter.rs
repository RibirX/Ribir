use std::ops::{Deref, DerefMut};

use ribir_algo::Resource;
use ribir_geom::{Angle, DeviceRect, Point, Rect, Size, Transform, Vector};
use serde::{Deserialize, Serialize};

use crate::{
  Brush, Color, Glyph, PixelImage, Svg, VisualGlyphs,
  color::{ColorFilterMatrix, LinearGradient, RadialGradient},
  font_db::FontDB,
  path::*,
  path_builder::PathBuilder,
};
/// The Painter provides you the ability to render 2D elements on a
/// two-dimensional canvas.
///
/// The coordinate (0, 0) is positioned at the upper-left corner of the canvas.
/// X-axis values progress toward the right edge of the canvas, while Y-axis
/// values increase towards the bottom edge of the canvas.
pub struct Painter {
  init_state: PainterState,
  state_stack: Vec<PainterState>,
  commands: Vec<PaintCommand>,
  path_builder: PathBuilder,
}

pub struct PainterResult<'a>(&'a mut Vec<PaintCommand>);

/// `PainterBackend` use to draw textures for every frame, All `draw_commands`
/// will called between `begin_frame` and `end_frame`
///
/// -- begin_frame()
///
///  +--> draw_commands --+
///  ^                    V
///  +----------<---------+
///                       
///
///
/// -+ end_frame()
///                       
pub trait PainterBackend {
  type Texture;

  /// Start a new frame, and clear the frame with `surface` color before draw.
  fn begin_frame(&mut self, surface: Color);

  /// Paint `commands` to the `output` Texture.  This may be called more than
  /// once during a frame.
  ///
  /// ## Undefined Behavior
  ///
  /// You should guarantee the output be same one in the same frame, otherwise
  /// it may cause undefined behavior.
  fn draw_commands(
    &mut self, viewport: DeviceRect, commands: &[PaintCommand], global_matrix: &Transform,
    output: &mut Self::Texture,
  );
  /// A frame end.
  fn end_frame(&mut self);
}

/// The enum of path types, which can be either shared or owned. This suggests
/// that if the path is shared among multiple commands, it can be cached for
/// efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintPath {
  Share(Resource<Path>),
  Own(Path),
}

/// The action to apply to the path, such as fill color, image, gradient, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintPathAction {
  Paint {
    brush: CommandBrush,
    /// The style to paint the path.
    painting_style: PaintingStyle,
  },

  Clip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandBrush {
  Color(Color),
  Image { img: Resource<PixelImage>, color_filter: ColorMatrix },
  Radial(RadialGradient),
  Linear(LinearGradient),
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum SpreadMethod {
  #[default]
  Pad,
  Reflect,
  Repeat,
}

/// A path and its geometry information are friendly to paint and cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCommand {
  /// The path to painting, and its axis is relative to the `bounds`.
  pub path: PaintPath,
  /// The bounds after path applied transform.
  pub paint_bounds: Rect,
  // The transform need to apply to the path.
  pub transform: Transform,
  // The action to apply to the path.
  pub action: PaintPathAction,
}

/// Explain the method for rendering shapes and paths, including filling or
/// stroking them.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum PaintingStyle {
  /// Fill the path.
  #[default]
  Fill,
  /// Stroke path with line width.
  Stroke(StrokeOptions),
}

/// Define the default method for the painter to render paths, including filling
/// or stroking them.
#[derive(Debug, Clone, Copy, Default)]
pub enum PathStyle {
  #[default]
  Fill,
  Stroke,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterType {
  Color(ColorFilterMatrix),
  Convolution(FlattenMatrix),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlattenMatrix {
  pub width: usize,
  pub height: usize,
  pub matrix: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintCommand {
  Path(PathCommand),
  PopClip,
  /// A Bundle of paint commands that can be assumed as a single command, that
  /// means the backend can cache it.
  Bundle {
    transform: Transform,
    color_filter: ColorMatrix,
    /// the bounds of the bundle commands. This is the union of all paint
    /// command
    bounds: Rect,
    cmds: Resource<Box<[PaintCommand]>>,
  },

  Filter {
    /// the path area to apply the filter.
    path: PaintPath,

    /// the transform to apply to the path.
    transform: Transform,

    /// the bounds of the path.
    path_bounds: Rect,

    /// the filter primitive to apply.
    filters: Vec<FilterType>,
  },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColorMatrix {
  Opacity(f32),
  Matrix(ColorFilterMatrix),
}

impl From<ColorFilterMatrix> for ColorMatrix {
  fn from(m: ColorFilterMatrix) -> Self { ColorMatrix::Matrix(m) }
}

impl Default for ColorMatrix {
  fn default() -> Self { ColorMatrix::Opacity(1.) }
}

impl ColorMatrix {
  pub fn identity() -> Self { ColorMatrix::Opacity(1.) }

  pub fn from_opacity(a: f32) -> Self { ColorMatrix::Opacity(a) }

  pub fn from_matrix(m: ColorFilterMatrix) -> Self { ColorMatrix::Matrix(m) }

  fn apply_alpha(&mut self, alpha: f32) {
    match self {
      ColorMatrix::Opacity(a) => {
        *a *= alpha;
      }
      ColorMatrix::Matrix(matrix) => {
        matrix.apply_alpha(alpha);
      }
    };
  }

  fn apply_color_filter(&mut self, mut matrix: ColorFilterMatrix) {
    let v = match self {
      ColorMatrix::Opacity(a) => {
        matrix.apply_alpha(*a);
        ColorMatrix::Matrix(matrix)
      }
      ColorMatrix::Matrix(m) => ColorMatrix::Matrix(m.chains(&matrix)),
    };
    *self = v;
  }

  pub fn chains(&mut self, next: &ColorMatrix) {
    match next {
      ColorMatrix::Opacity(a) => self.apply_alpha(*a),
      ColorMatrix::Matrix(m) => self.apply_color_filter(*m),
    }
  }

  pub fn apply_to(&self, color: &Color) -> Color {
    match self {
      ColorMatrix::Opacity(a) => color.apply_alpha(*a),
      ColorMatrix::Matrix(m) => m.apply_to(color),
    }
  }

  fn is_transparent(&self) -> bool {
    match self {
      ColorMatrix::Opacity(a) => *a == 0.,
      ColorMatrix::Matrix(m) => m.is_transparent(),
    }
  }

  pub fn to_matrix(&self) -> ColorFilterMatrix {
    match self {
      ColorMatrix::Opacity(a) => ColorFilterMatrix::only_alpha(*a),
      ColorMatrix::Matrix(m) => *m,
    }
  }
}

#[derive(Clone)]
struct PainterState {
  /// The line width use to stroke path.
  stroke_options: StrokeOptions,
  stroke_brush: Brush,
  fill_brush: Brush,
  style: PathStyle,
  transform: Transform,
  color_filter: ColorMatrix,
  clip_cnt: usize,
  /// The visible boundary of the painter in visual axis, not care about the
  /// transform.
  bounds: Rect,
}

impl PainterState {
  fn new(bounds: Rect) -> PainterState {
    PainterState {
      bounds,
      stroke_options: <_>::default(),
      stroke_brush: Color::BLACK.into(),
      fill_brush: Color::GRAY.into(),
      transform: Transform::identity(),
      clip_cnt: 0,
      color_filter: ColorMatrix::Opacity(1.),
      style: PathStyle::Fill,
    }
  }
}

impl Painter {
  pub fn new(viewport: Rect) -> Self {
    assert!(viewport.is_finite(), "viewport must be finite!");
    let init_state = PainterState::new(viewport);
    Self {
      state_stack: vec![init_state.clone()],
      init_state,
      commands: vec![],
      path_builder: Path::builder(),
    }
  }

  /// Creates an isolated state snapshot for atomic drawing operations.
  ///
  /// The forked painter maintains complete isolation from the original:
  /// - Captures cloned copy of the current rendering state
  /// - Maintains independent command buffer and path builder
  ///
  /// When merged back:
  /// - Inherits original state at fork time (immune to original's state
  ///   changes)
  /// - Transfers recorded commands without state contamination
  ///
  /// # Example
  /// ```
  /// use ribir_geom::*;
  /// use ribir_painter::*;
  ///
  /// let mut painter = Painter::new(Rect::from_size(Size::splat(512.)));
  /// let mut overlay = painter.fork(); // Capture initial clear state
  ///
  /// // Overlay draws in original state context
  /// overlay
  ///   .rect(&Rect::from_size(Size::splat(200.)))
  ///   .fill();
  ///
  /// // Original painter changes state
  /// painter
  ///   .set_fill_brush(Color::RED)
  ///   .circle(Point::splat(100.), 50.)
  ///   .fill();
  ///
  /// // Merge preserves overlay's commands with original fork-time state
  /// painter.merge(&mut overlay);
  /// ```
  pub fn fork(&self) -> Self {
    let init_state = self.current_state().clone();
    Painter {
      state_stack: vec![init_state.clone()],
      init_state,
      commands: vec![],
      path_builder: Path::builder(),
    }
  }

  /// Merges commands from a forked painter while preserving state isolation
  ///
  /// # Behavior
  /// - Transfers all drawing commands via buffer swapping (O(1) for Vec
  ///   storage)
  /// - Maintains original painter's current rendering state
  /// - Leaves forked painter with empty command buffer post-merge
  /// - Preserves both painters' path builders and state stacks
  pub fn merge(&mut self, forked: &mut Painter) { self.commands.append(forked.finish().0); }

  /// Change the default brush and text style of the painter, and then reset
  /// the painter state.
  pub fn set_init_state(&mut self, brush: Brush) {
    self.init_state.fill_brush = brush.clone();
    self.init_state.stroke_brush = brush;
    self.reset();
  }

  pub fn viewport(&self) -> &Rect { &self.init_state.bounds }

  /// Change the bounds of the painter can draw.But it won't take effect until
  /// the next time you call [`Painter::reset`]!.
  pub fn set_viewport(&mut self, bounds: Rect) { self.init_state.bounds = bounds; }

  pub fn intersection_paint_bounds(&self, rect: &Rect) -> Option<Rect> {
    self.paint_bounds().intersection(rect)
  }

  pub fn intersect_paint_bounds(&self, rect: &Rect) -> bool { self.paint_bounds().intersects(rect) }

  /// Returns the visible boundary of the painter in current state.
  pub fn paint_bounds(&self) -> Rect {
    let s = self.current_state();
    s.transform
      .inverse()
      .map_or_else(Rect::zero, |t| t.outer_transformed_rect(&s.bounds))
  }

  #[inline]
  pub fn finish(&mut self) -> PainterResult<'_> {
    self.fill_all_pop_clips();
    PainterResult(&mut self.commands)
  }

  /// Saves the entire state and return a guard to auto restore the state when
  /// if drop.
  #[must_use]
  pub fn save_guard(&mut self) -> PainterGuard<'_> {
    self.save();
    PainterGuard(self)
  }

  /// Saves the entire state of the canvas by pushing the current drawing state
  /// onto a stack.
  pub fn save(&mut self) -> &mut Self {
    let new_state = self.current_state().clone();
    self.state_stack.push(new_state);
    self
  }

  /// Restores the most recently saved canvas state by popping the top entry in
  /// the drawing state stack. If there is no saved state, this method does
  /// nothing.
  #[inline]
  pub fn restore(&mut self) {
    let clip_cnt = self.current_state().clip_cnt;
    self.state_stack.pop();
    self.push_n_pop_cmd(clip_cnt - self.current_state().clip_cnt);
  }

  pub fn reset(&mut self) {
    self.fill_all_pop_clips();
    self.commands.clear();
    self.state_stack.clear();
    self.state_stack.push(self.init_state.clone());
  }

  /// Return the brush used to stroke paths.
  #[inline]
  pub fn stroke_brush(&self) -> &Brush { &self.current_state().stroke_brush }

  /// Set the brush used to stroke paths.
  #[inline]
  pub fn set_stroke_brush(&mut self, brush: impl Into<Brush>) -> &mut Self {
    self.current_state_mut().stroke_brush = brush.into();
    self
  }

  /// Return the brush used to fill shapes.
  #[inline]
  pub fn fill_brush(&self) -> &Brush { &self.current_state().fill_brush }

  /// Set the brush used to fill shapes.
  #[inline]
  pub fn set_fill_brush(&mut self, brush: impl Into<Brush>) -> &mut Self {
    self.current_state_mut().fill_brush = brush.into();
    self
  }

  /// return the style for drawing the path.
  pub fn style(&self) -> PathStyle { self.current_state().style }

  /// Define the default style for drawing the path.
  pub fn set_style(&mut self, style: PathStyle) -> &mut Self {
    self.current_state_mut().style = style;
    self
  }

  pub fn apply_alpha(&mut self, alpha: f32) -> &mut Self {
    self
      .current_state_mut()
      .color_filter
      .apply_alpha(alpha);
    self
  }

  pub fn apply_color_filter(&mut self, filter: impl Into<ColorMatrix>) -> &mut Self {
    self
      .current_state_mut()
      .color_filter
      .chains(&filter.into());
    self
  }

  pub fn is_transparent(&self) -> bool { self.current_state().color_filter.is_transparent() }

  #[inline]
  pub fn set_strokes(&mut self, strokes: StrokeOptions) -> &mut Self {
    self.current_state_mut().stroke_options = strokes;
    self
  }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn line_width(&self) -> f32 { self.stroke_options().width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().stroke_options.width = line_width;
    self
  }

  #[inline]
  pub fn line_join(&self) -> LineJoin { self.stroke_options().line_join }

  #[inline]
  pub fn set_line_join(&mut self, line_join: LineJoin) -> &mut Self {
    self.current_state_mut().stroke_options.line_join = line_join;
    self
  }

  #[inline]
  pub fn line_cap(&mut self) -> LineCap { self.stroke_options().line_cap }

  #[inline]
  pub fn set_line_cap(&mut self, line_cap: LineCap) -> &mut Self {
    self.current_state_mut().stroke_options.line_cap = line_cap;
    self
  }

  #[inline]
  pub fn miter_limit(&self) -> f32 { self.stroke_options().miter_limit }

  #[inline]
  pub fn set_miter_limit(&mut self, miter_limit: f32) -> &mut Self {
    self
      .current_state_mut()
      .stroke_options
      .miter_limit = miter_limit;
    self
  }

  /// Return the current transformation matrix being applied to the layer.
  #[inline]
  pub fn transform(&self) -> &Transform { &self.current_state().transform }

  #[inline]
  fn color_filter(&self) -> &ColorMatrix { &self.current_state().color_filter }

  /// Resets (overrides) the current transformation to the identity matrix, and
  /// then invokes a transformation described by the arguments of this method.
  /// This lets you scale, rotate, translate (move), and skew the context.
  #[inline]
  pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
    self.current_state_mut().transform = transform;
    self
  }

  /// Apply this matrix to all subsequent paint commands。
  pub fn apply_transform(&mut self, transform: &Transform) -> &mut Self {
    let t = transform.then(self.transform());
    self.set_transform(t);
    self
  }

  pub fn clip(&mut self, path: PaintPath) -> &mut Self {
    invisible_return!(self);
    let p_bounds = path.bounds(None);
    if locatable_bounds(&p_bounds) {
      let intersect = self.intersection_paint_bounds(&p_bounds);
      let s = self.current_state_mut();
      if let Some(bounds) = intersect {
        s.bounds = s.transform.outer_transformed_rect(&bounds);
        let cmd = PathCommand::new(path, PaintPathAction::Clip, s.transform);
        self.commands.push(PaintCommand::Path(cmd));
        self.current_state_mut().clip_cnt += 1;
      } else {
        s.bounds = Rect::zero();
      }
    }

    self
  }

  /// Fill a path with fill brush.
  pub fn fill_path(&mut self, path: PaintPath) -> &mut Self {
    self.inner_draw_path(path, PathStyle::Fill)
  }

  /// Draw a path with the default style.
  pub fn draw_path(&mut self, path: PaintPath) -> &mut Self {
    self.inner_draw_path(path, self.current_state().style)
  }

  /// Outlines the current path with the current brush and `StrokeOptions`.
  pub fn stroke_path(&mut self, path: PaintPath) -> &mut Self {
    self.inner_draw_path(path, PathStyle::Stroke)
  }

  /// Strokes (outlines) the current path with the current brush and line width.
  pub fn stroke(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.stroke_path(builder.build().into())
  }

  /// Fill the current path with current brush.
  pub fn fill(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.fill_path(builder.build().into())
  }

  pub fn apply_color_matrix(&mut self, matrix: ColorFilterMatrix) -> &mut Self {
    self
      .current_state_mut()
      .color_filter
      .apply_color_filter(matrix);
    self
  }

  /// Draw the current path with the current paint style and brush.
  pub fn draw(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.draw_path(builder.build().into())
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
    let t = self.transform().pre_translate(Vector::new(x, y));
    self.set_transform(t);
    self
  }

  pub fn scale(&mut self, x: f32, y: f32) -> &mut Self {
    let t = self.transform().pre_scale(x, y);
    self.set_transform(t);
    self
  }

  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, at: Point) -> &mut Self {
    self.path_builder.begin_path(at);
    self
  }

  /// Tell the painter the sub-path is finished.
  #[inline]
  pub fn end_path(&mut self, close: bool) -> &mut Self {
    self.path_builder.end_path(close);
    self
  }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, to: Point) -> &mut Self {
    self.path_builder.line_to(to);
    self
  }

  /// Adds a cubic Bezier curve to the current path.
  #[inline]
  pub fn bezier_curve_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> &mut Self {
    self
      .path_builder
      .bezier_curve_to(ctrl1, ctrl2, to);
    self
  }

  /// Adds a quadratic Bézier curve to the current path.
  #[inline]
  pub fn quadratic_curve_to(&mut self, ctrl: Point, to: Point) -> &mut Self {
    self.path_builder.quadratic_curve_to(ctrl, to);
    self
  }

  /// adds a circular arc to the current sub-path, using the given control
  /// points and radius. The arc is automatically connected to the path's latest
  /// point with a straight line, if necessary for the specified
  #[inline]
  pub fn arc_to(
    &mut self, center: Point, radius: f32, start_angle: Angle, end_angle: Angle,
  ) -> &mut Self {
    self
      .path_builder
      .arc_to(center, radius, start_angle, end_angle);
    self
  }

  /// The ellipse_to() method creates an elliptical arc centered at `center`
  /// with the `radius`. The path starts at startAngle and ends at endAngle, and
  /// travels in the direction given by anticlockwise (defaulting to
  /// clockwise).
  #[inline]
  pub fn ellipse_to(
    &mut self, center: Point, radius: Vector, start_angle: Angle, end_angle: Angle,
  ) -> &mut Self {
    self
      .path_builder
      .ellipse_to(center, radius, start_angle, end_angle);
    self
  }

  /// Adds a sub-path containing a rectangle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn rect(&mut self, rect: &Rect) -> &mut Self {
    self.path_builder.rect(rect);
    self
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32) -> &mut Self {
    self.path_builder.circle(center, radius);
    self
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  /// #[inline]
  #[inline]
  pub fn rect_round(&mut self, rect: &Rect, radius: &Radius) -> &mut Self {
    self.path_builder.rect_round(rect, radius);
    self
  }

  /// Draws a bundle of paint commands that can be treated as a single command.
  /// This allows the backend to cache it.
  ///
  /// - **bounds** - The bounds of the bundle commands. This is the union of all
  ///   paint command bounds. It does not configure where the bundle is placed.
  ///   If you want to change the position of the bundle, you should call
  ///   `Painter::translate` before calling this method.
  /// - **cmds** - The list of paint commands to draw.
  pub fn draw_bundle_commands(
    &mut self, bounds: Rect, cmds: Resource<Box<[PaintCommand]>>,
  ) -> &mut Self {
    invisible_return!(self);
    if self.intersection_paint_bounds(&bounds).is_none() {
      return self;
    }

    let transform = *self.transform();
    let color_filter = *self.color_filter();
    let cmd = PaintCommand::Bundle { transform, color_filter, bounds, cmds };
    self.commands.push(cmd);
    self
  }

  pub fn draw_svg(&mut self, svg: &Svg) -> &mut Self {
    invisible_return!(self);
    let rect = Rect::from_size(svg.size());
    if self.intersection_paint_bounds(&rect).is_none() {
      return self;
    }

    let commands = svg.commands(self.fill_brush(), self.stroke_brush());

    // For a large number of path commands (more than 16), bundle them
    // together as a single resource. This allows the backend to cache
    // them collectively.
    // For a small number of path commands (less than 16), store them
    // individually as multiple resources. This means the backend doesn't
    // need to perform a single draw operation for an SVG.
    if commands.len() <= 16 {
      let transform = *self.transform();

      for cmd in commands.iter() {
        let cmd = match cmd.clone() {
          PaintCommand::Path(mut path) => {
            path.transform(&transform);
            if let PaintPathAction::Paint { ref mut brush, .. } = path.action {
              brush.apply_color_filter(self.color_filter());
            }
            PaintCommand::Path(path)
          }
          PaintCommand::PopClip => PaintCommand::PopClip,
          PaintCommand::Bundle { transform: b_ts, mut color_filter, bounds, cmds } => {
            color_filter.chains(self.color_filter());
            PaintCommand::Bundle { transform: transform.then(&b_ts), color_filter, bounds, cmds }
          }
          PaintCommand::Filter { .. } => cmd.clone(),
        };
        self.commands.push(cmd);
      }
    } else {
      self.draw_bundle_commands(rect, commands.clone());
    }

    self
  }

  /// Draw the image
  ///
  /// if src_rect is None then will draw the whole image fitted into dst_rect,
  /// otherwise will draw the partial src_rect of the image fitted into
  /// dst_rect.
  pub fn draw_img(
    &mut self, img: Resource<PixelImage>, dst_rect: &Rect, src_rect: &Option<Rect>,
  ) -> &mut Self {
    {
      let mut painter = self.save_guard();
      painter.translate(dst_rect.min_x(), dst_rect.min_y());

      let m_width = img.width() as f32;
      let m_height = img.height() as f32;
      let mut paint_rect = Rect::from_size(Size::new(m_width, m_height));
      if let Some(rc) = src_rect {
        assert!(paint_rect.contains_rect(rc));

        if paint_rect.width() != rc.width() || paint_rect.height() != rc.height() {
          painter.clip(Path::rect(&Rect::from_size(dst_rect.size)).into());
        }
        paint_rect = *rc;
      }
      painter
        .scale(dst_rect.width() / paint_rect.width(), dst_rect.height() / paint_rect.height())
        .translate(-paint_rect.min_x(), -paint_rect.min_y())
        .rect(&Rect::from_size(Size::new(m_width, m_height)))
        .set_fill_brush(img)
        .fill();
    }

    self
  }

  pub fn draw_glyph(&mut self, g: &Glyph, font_size: f32, font_db: &FontDB) -> &mut Self {
    #[inline]
    fn prefer_glyph_img_size(font_size: f32, matrix: &Transform) -> u16 {
      let scale = matrix.m11.max(matrix.m22);
      // We always prefer a larger image to has better quality for font.
      (font_size * scale * 2.) as u16
    }

    let Some(face) = font_db.try_get_face_data(g.face_id) else { return self };

    let unit = face.units_per_em() as f32;
    let matrix = *self.transform();

    let bounds = g.bounds();
    if let Some(svg) = face.glyph_svg_image(g.glyph_id) {
      let grid_scale = face
        .vertical_height()
        .map(|h| h as f32 / face.units_per_em() as f32)
        .unwrap_or(1.)
        .max(1.);
      let size = svg.size();
      let bound_size = bounds.size;
      let scale = (bound_size.width / size.width).min(bound_size.height / size.height) / grid_scale;
      self
        .translate(bounds.min_x(), bounds.min_y())
        .scale(scale, scale)
        .draw_svg(&svg);
    } else if let Some(img) =
      face.glyph_raster_image(g.glyph_id, prefer_glyph_img_size(font_size, &matrix))
    {
      let m_width = img.width() as f32;
      let m_height = img.height() as f32;
      let scale = (bounds.width() / m_width).min(bounds.height() / m_height);

      self
        .translate(bounds.min_x(), bounds.min_y())
        .scale(scale, scale)
        .draw_img(img, &Rect::from_size(Size::new(m_width, m_height)), &None);
    } else if let Some(path) = face.outline_glyph(g.glyph_id) {
      let scale = font_size / unit;
      self
        .translate(bounds.min_x(), bounds.min_y())
        .scale(scale, -scale)
        .translate(0., -unit)
        .draw_path(path.into());
    }

    self.set_transform(matrix);

    self
  }

  /// draw the text glyphs within the box_rect
  pub fn draw_glyphs_in_rect(
    self: &mut Painter, visual_glyphs: &VisualGlyphs, box_rect: Rect, font_db: &FontDB,
  ) -> &mut Self {
    let visual_rect = visual_glyphs.visual_rect();
    let Some(paint_rect) = self.intersection_paint_bounds(&box_rect) else {
      return self;
    };
    let Some(glyphs) = visual_glyphs.glyphs_in_bounds(&paint_rect) else {
      return self;
    };

    self.translate(visual_rect.origin.x, visual_rect.origin.y);

    for g in glyphs {
      self.draw_glyph(&g, visual_glyphs.font_size(), font_db);
    }

    self
  }

  pub fn filters(&mut self, path: PaintPath, filters: Vec<FilterType>) -> &mut Self {
    invisible_return!(self);
    let p_bounds = path.bounds(None);
    if p_bounds.is_empty() || !locatable_bounds(&p_bounds) {
      return self;
    }

    if !self.intersect_paint_bounds(&p_bounds) {
      return self;
    }

    let transform = *self.transform();
    let path_bounds = transform.outer_transformed_rect(&p_bounds);
    self
      .commands
      .push(PaintCommand::Filter { path, path_bounds, transform, filters });

    self
  }

  fn inner_draw_path(&mut self, path: PaintPath, path_style: PathStyle) -> &mut Self {
    invisible_return!(self);
    let line_width = matches!(path_style, PathStyle::Stroke).then(|| self.line_width());
    let p_bounds = path.bounds(line_width);
    if p_bounds.is_empty()
      || !locatable_bounds(&p_bounds)
      || !self.intersect_paint_bounds(&p_bounds)
    {
      return self;
    }

    let brush = match path_style {
      PathStyle::Fill => self.fill_brush().clone(),
      PathStyle::Stroke => self.stroke_brush().clone(),
    };

    if brush.is_visible() {
      let mut brush = CommandBrush::from(brush);
      let painting_style = match path_style {
        PathStyle::Fill => PaintingStyle::Fill,
        PathStyle::Stroke => PaintingStyle::Stroke(self.stroke_options().clone()),
      };
      brush.apply_color_filter(self.color_filter());
      let ts = *self.transform();
      let action = PaintPathAction::Paint { brush, painting_style };
      let cmd = PathCommand::new(path, action, ts);
      self.commands.push(PaintCommand::Path(cmd));
    }

    self
  }
}

impl PaintingStyle {
  pub fn line_width(&self) -> Option<f32> {
    match self {
      PaintingStyle::Fill => None,
      PaintingStyle::Stroke(stroke) => Some(stroke.width),
    }
  }
}

impl Painter {
  fn current_state(&self) -> &PainterState {
    self
      .state_stack
      .last()
      .expect("Must have one state in stack!")
  }

  fn current_state_mut(&mut self) -> &mut PainterState {
    self
      .state_stack
      .last_mut()
      .expect("Must have one state in stack!")
  }

  fn stroke_options(&self) -> &StrokeOptions { &self.current_state().stroke_options }

  fn push_n_pop_cmd(&mut self, n: usize) {
    for _ in 0..n {
      if matches!(
        self.commands.last(),
        Some(PaintCommand::Path(PathCommand { action: PaintPathAction::Clip, .. }))
      ) {
        self.commands.pop();
      } else {
        self.commands.push(PaintCommand::PopClip)
      }
    }
  }

  fn fill_all_pop_clips(&mut self) {
    let clip_cnt = self.current_state().clip_cnt;
    self
      .state_stack
      .iter_mut()
      .for_each(|s| s.clip_cnt = 0);
    self.push_n_pop_cmd(clip_cnt);
  }

  fn is_visible_canvas(&self) -> bool {
    let t = self.current_state().transform;
    !self.is_transparent()
      && locatable_bounds(self.viewport())
      && t.m11.is_finite()
      && t.m12.is_finite()
      && t.m21.is_finite()
      && t.m22.is_finite()
      && t.m31.is_finite()
      && t.m32.is_finite()
  }
}

impl From<Brush> for CommandBrush {
  fn from(brush: Brush) -> Self {
    match brush {
      Brush::Color(color) => CommandBrush::Color(color),
      Brush::Image(img) => CommandBrush::Image { img, color_filter: ColorMatrix::default() },
      Brush::RadialGradient(radial_gradient) => CommandBrush::Radial(radial_gradient),
      Brush::LinearGradient(linear_gradient) => CommandBrush::Linear(linear_gradient),
    }
  }
}

impl Drop for PainterResult<'_> {
  fn drop(&mut self) { self.0.clear() }
}

impl<'a> Deref for PainterResult<'a> {
  type Target = [PaintCommand];
  fn deref(&self) -> &Self::Target { self.0 }
}

impl<'a> DerefMut for PainterResult<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

/// An RAII implementation of a "scoped state" of the render layer.
///
/// When this structure is dropped (falls out of scope), changed state will auto
/// restore. The data can be accessed through this guard via its Deref and
/// DerefMut implementations.
pub struct PainterGuard<'a>(&'a mut Painter);

impl<'a> Drop for PainterGuard<'a> {
  #[inline]
  fn drop(&mut self) {
    debug_assert!(!self.0.state_stack.is_empty());
    self.0.restore();
  }
}

impl<'a> Deref for PainterGuard<'a> {
  type Target = Painter;
  #[inline]
  fn deref(&self) -> &Self::Target { self.0 }
}

impl<'a> DerefMut for PainterGuard<'a> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

impl Deref for PaintPath {
  type Target = Path;
  fn deref(&self) -> &Self::Target {
    match self {
      PaintPath::Share(p) => p.deref(),
      PaintPath::Own(p) => p,
    }
  }
}

impl From<Path> for PaintPath {
  fn from(p: Path) -> Self { PaintPath::Own(p) }
}

impl From<Resource<Path>> for PaintPath {
  fn from(p: Resource<Path>) -> Self { PaintPath::Share(p) }
}

impl PathCommand {
  pub fn new(path: PaintPath, action: PaintPathAction, transform: Transform) -> Self {
    let line_width = if let PaintPathAction::Paint { painting_style, .. } = &action {
      painting_style.line_width()
    } else {
      None
    };
    let paint_bounds = transform.outer_transformed_rect(&path.bounds(line_width));
    Self { path, transform, paint_bounds, action }
  }

  pub fn scale(&mut self, scale: f32) {
    self.transform = self.transform.then_scale(scale, scale);
    self.paint_bounds = self.paint_bounds.scale(scale, scale);
  }

  pub fn transform(&mut self, transform: &Transform) {
    self.transform = self.transform.then(transform);
    self.paint_bounds = self
      .transform
      .outer_transformed_rect(&self.path.bounds(None));
  }
}

impl CommandBrush {
  pub fn apply_color_filter(&mut self, filter: &ColorMatrix) -> &mut Self {
    match self {
      CommandBrush::Color(color) => *color = filter.apply_to(color),
      CommandBrush::Image { color_filter, .. } => color_filter.chains(filter),
      CommandBrush::Radial(RadialGradient { stops, .. })
      | CommandBrush::Linear(LinearGradient { stops, .. }) => stops
        .iter_mut()
        .for_each(|s| s.color = filter.apply_to(&s.color)),
    }
    self
  }
}

impl From<usvg::SpreadMethod> for SpreadMethod {
  fn from(value: usvg::SpreadMethod) -> Self {
    match value {
      usvg::SpreadMethod::Pad => SpreadMethod::Pad,
      usvg::SpreadMethod::Reflect => SpreadMethod::Reflect,
      usvg::SpreadMethod::Repeat => SpreadMethod::Repeat,
    }
  }
}

// bounds that has a limited location and size
fn locatable_bounds(bounds: &Rect) -> bool {
  bounds.origin.is_finite() && !bounds.width().is_nan() && !bounds.height().is_nan()
}

macro_rules! invisible_return {
  ($this:ident) => {
    if !$this.is_visible_canvas() {
      return $this;
    }
  };
}
use invisible_return;

#[cfg(test)]
mod test {
  use ribir_geom::rect;

  use super::*;

  fn painter() -> Painter { Painter::new(Rect::from_size(Size::new(512., 512.))) }

  #[test]
  fn save_guard() {
    let mut painter = painter();
    {
      let mut guard = painter.save_guard();
      let t = Transform::new(1., 1., 1., 1., 1., 1.);
      guard.set_transform(t);
      assert_eq!(&t, guard.transform());
      {
        let mut p2 = guard.save_guard();
        let t2 = Transform::new(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.transform());
      }
      assert_eq!(&t, guard.transform());
    }
    assert_eq!(&Transform::new(1., 0., 0., 1., 0., 0.), painter.transform());
  }

  #[test]
  fn fix_clip_pop_without_restore() {
    let mut painter = painter();
    let commands = painter
      .save()
      .clip(Path::rect(&rect(0., 0., 100., 100.)).into())
      .rect(&rect(0., 0., 10., 10.))
      .fill()
      .save()
      .clip(Path::rect(&rect(0., 0., 50., 50.)).into())
      .rect(&rect(0., 0., 10., 10.))
      .fill()
      .finish();

    assert!(matches!(commands[commands.len() - 1], PaintCommand::PopClip));
    assert!(matches!(commands[commands.len() - 2], PaintCommand::PopClip));

    std::mem::drop(commands);

    assert_eq!(painter.current_state().clip_cnt, 0);
  }

  #[test]
  fn filter_invalid_clip() {
    let mut painter = painter();

    painter
      .save()
      .set_transform(Transform::translation(f32::NAN, f32::INFINITY))
      .clip(Path::rect(&rect(0., 0., 10., 10.)).into());
    assert_eq!(painter.commands.len(), 0);
  }

  #[test]
  fn filter_invalid_commands() {
    let mut painter = painter();

    let svg =
      Svg::parse_from_bytes(include_bytes!("../../tests/assets/test1.svg"), true, false).unwrap();
    painter
      .save()
      .set_transform(Transform::translation(f32::NAN, f32::INFINITY))
      .draw_svg(&svg);
    assert_eq!(painter.commands.len(), 0);
  }

  #[test]
  fn draw_svg_gradient() {
    let mut painter = Painter::new(Rect::from_size(Size::new(64., 64.)));
    let svg = Svg::parse_from_bytes(
      include_bytes!("../../tests/assets/fill_with_gradient.svg"),
      true,
      false,
    )
    .unwrap();

    painter.draw_svg(&svg);
  }

  #[test]
  fn fix_incorrect_bounds_axis() {
    let mut painter = painter();

    painter
      .save()
      .clip(Path::rect(&rect(0., 0., 100., 100.)).into())
      .set_transform(Transform::translation(500., 500.))
      .rect(&rect(-500., -500., 10., 10.))
      .fill();
    assert_eq!(painter.commands.len(), 2);
  }

  #[test]
  fn fix_scale_zero_crash() {
    let mut painter = painter();

    painter
      .scale(0., 0.)
      .rect(&rect(0., 0., 10., 10.))
      .fill();
  }

  #[test]
  fn clip_zero() {
    let mut painter = painter();

    painter
      .clip(Path::rect(&rect(0., 0., 0., 0.)).into())
      .rect(&rect(0., 0., 10., 10.))
      .fill();

    assert_eq!(painter.commands.len(), 0);

    painter.draw_bundle_commands(
      Rect::from_size(Size::new(10., 10.)),
      Resource::new(Box::new([PaintCommand::Path(PathCommand::new(
        Path::rect(&rect(0., 0., 10., 10.)).into(),
        PaintPathAction::Paint {
          painting_style: PaintingStyle::Fill,
          brush: Brush::Color(Color::BLACK).into(),
        },
        Transform::identity(),
      ))])),
    );
    assert_eq!(painter.commands.len(), 0);
  }
}
