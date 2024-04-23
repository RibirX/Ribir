use std::ops::{Deref, DerefMut};

use ribir_algo::ShareResource;
use ribir_geom::{Angle, DeviceRect, Point, Rect, Size, Transform, Vector};
use serde::{Deserialize, Serialize};

use crate::{
  color::{LinearGradient, RadialGradient},
  path::*,
  path_builder::PathBuilder,
  Brush, Color, PixelImage, Svg,
};
/// The painter is a two-dimensional grid. The coordinate (0, 0) is at the
/// upper-left corner of the canvas. Along the X-axis, values increase towards
/// the right edge of the canvas. Along the Y-axis, values increase towards the
/// bottom edge of the canvas.
// #[derive(Default, Debug, Clone)]
pub struct Painter {
  viewport: Rect,
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
    &mut self, viewport: DeviceRect, commands: Vec<PaintCommand>, output: &mut Self::Texture,
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
  ColorPath { path: PaintPath, color: Color },
  ImgPath { path: PaintPath, img: ShareResource<PixelImage>, opacity: f32 },
  RadialGradient { path: PaintPath, radial_gradient: RadialGradient },
  LinearGradient { path: PaintPath, linear_gradient: LinearGradient },
  // Todo: keep rectangle clip.
  Clip(PaintPath),
  PopClip,
}

impl PaintCommand {
  pub fn transform(&mut self, transform: &Transform) -> &mut Self {
    match self {
      PaintCommand::ColorPath { path, .. }
      | PaintCommand::ImgPath { path, .. }
      | PaintCommand::RadialGradient { path, .. }
      | PaintCommand::LinearGradient { path, .. }
      | PaintCommand::Clip(path) => path.transform(transform),
      PaintCommand::PopClip => {}
    }
    self
  }

  pub fn apply_alpha(&mut self, alpha: f32) -> &mut Self {
    match self {
      PaintCommand::ColorPath { color, .. } => *color = color.apply_alpha(alpha),
      PaintCommand::ImgPath { opacity, .. } => *opacity *= alpha,
      PaintCommand::RadialGradient { radial_gradient, .. } => {
        radial_gradient
          .stops
          .iter_mut()
          .for_each(|s| s.color = s.color.apply_alpha(alpha));
      }
      PaintCommand::LinearGradient { linear_gradient, .. } => {
        linear_gradient
          .stops
          .iter_mut()
          .for_each(|s| s.color = s.color.apply_alpha(alpha));
      }
      PaintCommand::Clip(_) | PaintCommand::PopClip => {}
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
  /// The visible boundary of the painter in visual axis, not care about the
  /// transform.
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
  pub fn new(viewport: Rect) -> Self {
    assert!(viewport.is_finite(), "viewport must be finite!");
    Self {
      state_stack: vec![PainterState::new(viewport)],
      commands: vec![],
      path_builder: Path::builder(),
      viewport,
    }
  }

  pub fn viewport(&self) -> &Rect { &self.viewport }

  /// Change the bounds of the painter can draw.But it won't take effect until
  /// the next time you call [`Painter::reset`]!.
  pub fn set_viewport(&mut self, bounds: Rect) { self.viewport = bounds; }

  pub fn intersection_paint_bounds(&self, rect: &Rect) -> Option<Rect> {
    self.paint_bounds().intersection(rect)
  }

  pub fn intersect_paint_bounds(&self, rect: &Rect) -> bool { self.paint_bounds().intersects(rect) }

  /// Returns the visible boundary of the painter in current state.
  pub fn paint_bounds(&self) -> Rect {
    let s = self.current_state();
    s.transform
      .inverse()
      .unwrap()
      .outer_transformed_rect(&s.bounds)
  }

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
    self
      .state_stack
      .push(PainterState::new(self.viewport));
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
    self
      .current_state_mut()
      .stroke_options
      .miter_limit = miter_limit;
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
    invisible_return!(self);

    if locatable_bounds(&path.bounds) {
      if let Some(bounds) = self.intersection_paint_bounds(&path.bounds) {
        let s = self.current_state_mut();
        s.bounds = s.transform.outer_transformed_rect(&bounds);

        let paint_path = PaintPath::new(path, s.transform);
        self.commands.push(PaintCommand::Clip(paint_path));
        self.current_state_mut().clip_cnt += 1;
      }
    }

    self
  }

  /// Fill a path with its style.
  pub fn fill_path(&mut self, p: Path) -> &mut Self {
    invisible_return!(self);

    if locatable_bounds(&p.bounds)
      && self.intersect_paint_bounds(&p.bounds)
      && self.is_visible_brush()
    {
      let ts = *self.get_transform();
      let path = PaintPath::new(p, ts);
      let mut cmd = match self.current_state().brush.clone() {
        Brush::Color(color) => PaintCommand::ColorPath { path, color },
        Brush::Image(img) => PaintCommand::ImgPath { path, img, opacity: 1. },
        Brush::RadialGradient(radial_gradient) => {
          PaintCommand::RadialGradient { path, radial_gradient }
        }
        Brush::LinearGradient(linear_gradient) => {
          PaintCommand::LinearGradient { path, linear_gradient }
        }
      };
      cmd.apply_alpha(self.alpha());
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
    let t = self
      .get_transform()
      .pre_translate(Vector::new(x, y));
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

  pub fn draw_svg(&mut self, svg: &Svg) -> &mut Self {
    invisible_return!(self);

    let transform = *self.get_transform();
    let alpha = self.alpha();

    for cmd in svg.paint_commands.iter() {
      let mut cmd = cmd.clone();
      cmd.transform(&transform).apply_alpha(alpha);
      self.commands.push(cmd);
    }

    self
  }

  /// Draw the image
  ///
  /// if src_rect is None then will draw the whole image fitted into dst_rect,
  /// otherwise will draw the partial src_rect of the image fitted into
  /// dst_rect.
  pub fn draw_img(
    &mut self, img: ShareResource<PixelImage>, dst_rect: &Rect, src_rect: &Option<Rect>,
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
        .scale(dst_rect.width() / paint_rect.width(), dst_rect.height() / paint_rect.height())
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
    self
      .state_stack
      .iter_mut()
      .for_each(|s| s.clip_cnt = 0);
    self.push_n_pop_cmd(clip_cnt);
  }

  fn is_visible_canvas(&self) -> bool {
    let t = self.current_state().transform;
    self.alpha() > 0.
      && locatable_bounds(self.viewport())
      && t.m11.is_finite()
      && t.m12.is_finite()
      && t.m21.is_finite()
      && t.m22.is_finite()
      && t.m31.is_finite()
      && t.m32.is_finite()
  }

  fn is_visible_brush(&self) -> bool {
    match self.current_state().brush {
      Brush::Color(c) => c.alpha > 0,
      Brush::Image(_) => true,
      Brush::RadialGradient(RadialGradient { ref stops, .. })
      | Brush::LinearGradient(LinearGradient { ref stops, .. }) => {
        stops.iter().any(|s| s.color.alpha > 0)
      }
    }
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
    if transform
      .outer_transformed_rect(path.bounds())
      .width()
      .is_nan()
    {
      println!("paint_bounds.width().is_nan()");
    }
    let paint_bounds = transform.outer_transformed_rect(path.bounds());
    PaintPath { path, transform, paint_bounds }
  }

  pub fn scale(&mut self, scale: f32) {
    self.transform = self.transform.then_scale(scale, scale);
    self.paint_bounds = self.paint_bounds.scale(scale, scale);
  }

  pub fn transform(&mut self, transform: &Transform) {
    self.transform = self.transform.then(transform);
    self.paint_bounds = self
      .transform
      .outer_transformed_rect(self.path.bounds());
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
      assert_eq!(&t, guard.get_transform());
      {
        let mut p2 = guard.save_guard();
        let t2 = Transform::new(2., 2., 2., 2., 2., 2.);
        p2.set_transform(t2);
        assert_eq!(&t2, p2.get_transform());
      }
      assert_eq!(&t, guard.get_transform());
    }
    assert_eq!(&Transform::new(1., 0., 0., 1., 0., 0.), painter.get_transform());
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

    assert!(matches!(commands[commands.len() - 1], PaintCommand::PopClip));
    assert!(matches!(commands[commands.len() - 2], PaintCommand::PopClip));
  }

  #[test]
  fn filter_invalid_clip() {
    let mut painter = painter();

    painter
      .save()
      .set_transform(Transform::translation(f32::NAN, f32::INFINITY))
      .clip(Path::rect(&rect(0., 0., 10., 10.)));
    assert_eq!(painter.commands.len(), 0);
  }

  #[test]
  fn filter_invalid_commands() {
    let mut painter = painter();

    let svg = Svg::parse_from_bytes(include_bytes!("../../tests/assets/test1.svg")).unwrap();
    painter
      .save()
      .set_transform(Transform::translation(f32::NAN, f32::INFINITY))
      .draw_svg(&svg);
    assert_eq!(painter.commands.len(), 0);
  }

  #[test]
  fn draw_svg_gradient() {
    let mut painter = Painter::new(Rect::from_size(Size::new(64., 64.)));
    let svg =
      Svg::parse_from_bytes(include_bytes!("../../tests/assets/fill_with_gradient.svg")).unwrap();

    painter.draw_svg(&svg);
  }

  #[test]
  fn fix_incorrect_bounds_axis() {
    let mut painter = painter();

    painter
      .save()
      .clip(Path::rect(&rect(0., 0., 100., 100.)))
      .set_transform(Transform::translation(500., 500.))
      .rect(&rect(-500., -500., 10., 10.))
      .fill();
    assert_eq!(painter.commands.len(), 2);
  }
}
