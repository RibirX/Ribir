use crate::{
  color::{LinearGradient, RadialGradient},
  path::*,
  path_builder::PathBuilder,
  Brush, Color, PixelImage, Svg,
};
use ribir_algo::ShareResource;
use ribir_geom::{Angle, DeviceRect, Point, Rect, Size, Transform, Vector};

use serde::{Deserialize, Serialize};

use std::ops::{Deref, DerefMut};
/// The painter is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
// #[derive(Default, Debug, Clone)]
pub struct Painter {
  bounds: Rect,
  state_stack: Vec<PainterState>,
  commands: Vec<PaintCommand>,
  path_builder: PathBuilder,
}

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

  fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing);

  /// A frame start.
  fn begin_frame(&mut self);

  /// Paint `commands` to the `output` Texture.
  /// This may be called more than once during a frame.
  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    surface: Color,
    output: &mut Self::Texture,
  );

  /// A frame end.
  fn end_frame(&mut self);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

/// A path and its geometry information are friendly to paint and cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintPath {
  /// The path to painting, and its axis is relative to the `bounds`.
  pub path: Path,
  /// The bounds after path applied transform.
  pub paint_bounds: Rect,
  // The transform need to apply to the path.
  pub transform: Transform,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum SpreadMethod {
  #[default]
  Pad,
  Reflect,
  Repeat,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintCommand {
  ColorPath {
    path: PaintPath,
    color: Color,
  },
  ImgPath {
    path: PaintPath,
    img: ShareResource<PixelImage>,
    opacity: f32,
  },
  RadialGradient {
    path: PaintPath,
    radial_gradient: RadialGradient,
  },
  LinearGradient {
    path: PaintPath,
    linear_gradient: LinearGradient,
  },
  // Todo: keep rectangle clip.
  Clip(PaintPath),
  PopClip,
}

impl PaintCommand {
  pub fn transform(mut self, transform: &Transform) -> Self {
    match &mut self {
      PaintCommand::ColorPath { path, .. }
      | PaintCommand::ImgPath { path, .. }
      | PaintCommand::RadialGradient { path, .. }
      | PaintCommand::LinearGradient { path, .. }
      | PaintCommand::Clip(path) => path.transform(transform),
      PaintCommand::PopClip => {}
    }
    self
  }
}

#[derive(Clone)]
struct PainterState {
  /// The line width use to stroke path.
  stroke_options: StrokeOptions,
  brush: Brush,
  transform: Transform,
  opacity: f32,
  clip_cnt: usize,

  /// The bounds without transform.
  bounds: Rect,
}

impl PainterState {
  fn new(bounds: Rect) -> PainterState {
    PainterState {
      bounds,
      stroke_options: <_>::default(),
      brush: Color::BLACK.into(),
      transform: Transform::identity(),
      clip_cnt: 0,
      opacity: 1.,
    }
  }
}

impl Painter {
  pub fn new(bounds: Rect) -> Self {
    Self {
      state_stack: vec![PainterState::new(bounds)],
      commands: vec![],
      path_builder: Path::builder(),
      bounds,
    }
  }

  /// Change the bounds of the painter can draw.But it won't take effect until
  /// the next time you call [`Painter::reset`]!.
  pub fn set_bounds(&mut self, bounds: Rect) { self.bounds = bounds; }

  pub fn rect_in_paint_bounds(&self, rect: &Rect) -> Option<Rect> {
    self.get_transform().inverse().and_then(|trans| {
      trans
        .outer_transformed_rect(self.paint_bounds())
        .intersection(rect)
    })
  }

  pub fn paint_bounds(&self) -> &Rect { &self.current_state().bounds }

  #[inline]
  pub fn finish(&mut self) -> Vec<PaintCommand> {
    self.fill_all_pop_clips();
    let commands = self.commands.clone();
    self.commands.clear();
    commands
  }

  /// Saves the entire state and return a guard to auto restore the state when
  /// if drop.
  #[must_use]
  pub fn save_guard(&mut self) -> PainterGuard {
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
    self.state_stack.push(PainterState::new(self.bounds));
  }

  /// Returns the color, gradient, or pattern used for draw. Only `Color`
  /// support now.
  #[inline]
  pub fn get_brush(&self) -> &Brush { &self.current_state().brush }

  /// Change the style of pen that used to draw path.
  #[inline]
  pub fn set_brush<S: Into<Brush>>(&mut self, brush: S) -> &mut Self {
    self.current_state_mut().brush = brush.into();
    self
  }

  pub fn apply_alpha(&mut self, alpha: f32) -> &mut Self {
    self.current_state_mut().opacity *= alpha;
    self
  }

  pub fn alpha(&self) -> f32 { self.current_state().opacity }

  #[inline]
  pub fn set_strokes(&mut self, strokes: StrokeOptions) -> &mut Self {
    self.current_state_mut().stroke_options = strokes;
    self
  }

  /// Return the line width of the stroke pen.
  #[inline]
  pub fn get_line_width(&self) -> f32 { self.stroke_options().width }

  /// Set the line width of the stroke pen with `line_width`
  #[inline]
  pub fn set_line_width(&mut self, line_width: f32) -> &mut Self {
    self.current_state_mut().stroke_options.width = line_width;
    self
  }

  #[inline]
  pub fn get_line_join(&self) -> LineJoin { self.stroke_options().line_join }

  #[inline]
  pub fn set_line_join(&mut self, line_join: LineJoin) -> &mut Self {
    self.current_state_mut().stroke_options.line_join = line_join;
    self
  }

  #[inline]
  pub fn get_line_cap(&mut self) -> LineCap { self.stroke_options().line_cap }

  #[inline]
  pub fn set_line_cap(&mut self, line_cap: LineCap) -> &mut Self {
    self.current_state_mut().stroke_options.line_cap = line_cap;
    self
  }

  #[inline]
  pub fn get_miter_limit(&self) -> f32 { self.stroke_options().miter_limit }

  #[inline]
  pub fn set_miter_limit(&mut self, miter_limit: f32) -> &mut Self {
    self.current_state_mut().stroke_options.miter_limit = miter_limit;
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

  /// Apply this matrix to all subsequent paint commands。
  pub fn apply_transform(&mut self, transform: &Transform) -> &mut Self {
    let t = transform.then(self.get_transform());
    self.set_transform(t);
    self
  }

  pub fn clip(&mut self, path: Path) -> &mut Self {
    let paint_path = PaintPath::new(path, *self.get_transform());

    self.current_state_mut().bounds = self
      .current_state()
      .bounds
      .intersection(&paint_path.paint_bounds)
      .unwrap_or(Rect::zero());

    self.commands.push(PaintCommand::Clip(paint_path));
    self.current_state_mut().clip_cnt += 1;
    self
  }

  /// Fill a path with its style.
  pub fn fill_path(&mut self, p: Path) -> &mut Self {
    let ts = *self.get_transform();
    let path = PaintPath::new(p, ts);
    if !path.paint_bounds.is_empty() && path.paint_bounds.intersects(self.paint_bounds()) {
      let opacity = self.alpha();
      let cmd = match self.current_state().brush.clone() {
        Brush::Color(color) => PaintCommand::ColorPath {
          path,
          color: color.apply_alpha(opacity),
        },
        Brush::Image(img) => PaintCommand::ImgPath { path, img, opacity },
        Brush::RadialGradient(radial_gradient) => {
          PaintCommand::RadialGradient { path, radial_gradient }
        }
        Brush::LinearGradient(linear_gradient) => {
          PaintCommand::LinearGradient { path, linear_gradient }
        }
      };
      self.commands.push(cmd);
    }

    self
  }

  /// Fill a path with its style.
  pub fn stroke_path(&mut self, path: Path) -> &mut Self {
    if let Some(stroke_path) = path.stroke(self.stroke_options(), Some(self.get_transform())) {
      self.fill_path(stroke_path);
    }
    self
  }

  /// Strokes (outlines) the current path with the current brush and line width.
  pub fn stroke(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.stroke_path(builder.build())
  }

  /// Fill the current path with current brush.
  pub fn fill(&mut self) -> &mut Self {
    let builder = std::mem::take(&mut self.path_builder);
    self.fill_path(builder.build())
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
    let t = self.get_transform().pre_translate(Vector::new(x, y));
    self.set_transform(t);
    self
  }

  pub fn scale(&mut self, x: f32, y: f32) -> &mut Self {
    let t = self.get_transform().pre_scale(x, y);
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
    self.path_builder.bezier_curve_to(ctrl1, ctrl2, to);
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
    &mut self,
    center: Point,
    radius: f32,
    start_angle: Angle,
    end_angle: Angle,
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
    &mut self,
    center: Point,
    radius: Vector,
    start_angle: Angle,
    end_angle: Angle,
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

  pub fn draw_svg(&mut self, svg: &Svg) -> &mut Self {
    let transform = *self.get_transform();
    svg
      .paint_commands
      .iter()
      .for_each(|c| self.commands.push(c.clone().transform(&transform)));
    self
  }

  /// Draw the image
  ///
  /// if src_rect is None then will draw the whole image fitted into dst_rect,
  /// otherise will draw the partial src_rect of the image fitted into dst_rect.
  pub fn draw_img(
    &mut self,
    img: ShareResource<PixelImage>,
    dst_rect: &Rect,
    src_rect: &Option<Rect>,
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
          painter.clip(Path::rect(&Rect::from_size(dst_rect.size)));
        }
        paint_rect = *rc;
      }
      painter
        .scale(
          dst_rect.width() / paint_rect.width(),
          dst_rect.height() / paint_rect.height(),
        )
        .translate(-paint_rect.min_x(), -paint_rect.min_y())
        .rect(&Rect::from_size(Size::new(m_width, m_height)))
        .set_brush(img)
        .fill();
    }

    self
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
      if matches!(self.commands.last(), Some(PaintCommand::Clip(_))) {
        self.commands.pop();
      } else {
        self.commands.push(PaintCommand::PopClip)
      }
    }
  }

  fn fill_all_pop_clips(&mut self) {
    let clip_cnt = self.current_state().clip_cnt;
    self.state_stack.iter_mut().for_each(|s| s.clip_cnt = 0);
    self.push_n_pop_cmd(clip_cnt);
  }
}

/// An RAII implementation of a "scoped state" of the render layer. When this
/// structure is dropped (falls out of scope), changed state will auto restore.
/// The data can be accessed through this guard via its Deref and DerefMut
/// implementations.
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

impl PaintPath {
  pub fn new(path: Path, transform: Transform) -> Self {
    let paint_bounds = transform.outer_transformed_rect(path.bounds());
    PaintPath { path, transform, paint_bounds }
  }

  pub fn scale(&mut self, scale: f32) {
    self.transform = self.transform.then_scale(scale, scale);
    self.paint_bounds = self.paint_bounds.scale(scale, scale);
  }

  pub fn transform(&mut self, transform: &Transform) {
    self.transform = self.transform.then(transform);
    self.paint_bounds = self.transform.outer_transformed_rect(self.path.bounds());
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use ribir_geom::{rect, Size};

  fn painter() -> Painter { Painter::new(Rect::from_size(Size::new(512., 512.))) }

  #[test]
  fn save_guard() {
    let mut painter = painter();
    {
      let mut guard = painter.save_guard();
      let t = Transform::new(1., 1., 1., 1., 1., 1.);
      guard.set_transform(t);
      assert_eq!(&t, guard.get_transform());
      {
        let mut p2 = guard.save_guard();
        let t2 = Transform::new(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.get_transform());
      }
      assert_eq!(&t, guard.get_transform());
    }
    assert_eq!(
      &Transform::new(1., 0., 0., 1., 0., 0.),
      painter.get_transform()
    );
  }

  #[test]
  fn fix_clip_pop_without_restore() {
    let mut painter = painter();
    let commands = painter
      .save()
      .clip(Path::rect(&rect(0., 0., 100., 100.)))
      .rect(&rect(0., 0., 10., 10.))
      .fill()
      .save()
      .clip(Path::rect(&rect(0., 0., 50., 50.)))
      .rect(&rect(0., 0., 10., 10.))
      .fill()
      .finish();

    assert_eq!(painter.current_state().clip_cnt, 0);

    assert!(matches!(
      commands[commands.len() - 1],
      PaintCommand::PopClip
    ));
    assert!(matches!(
      commands[commands.len() - 2],
      PaintCommand::PopClip
    ));
  }
}
