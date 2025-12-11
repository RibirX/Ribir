use lyon_algorithms::path::{
  Winding,
  builder::BorderRadii,
  geom::{Arc, LineSegment},
  path::Builder as LyonBuilder,
};
use ribir_geom::{Angle, Point, Rect, Vector};

use crate::{Path, Radius};

#[derive(Default)]
pub struct PathBuilder {
  pub(crate) lyon_builder: LyonBuilder,
}

impl PathBuilder {
  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, at: Point) -> &mut Self {
    self.lyon_builder.begin(at.to_untyped());
    self
  }

  /// Tell the builder the sub-path is finished.
  /// if `close` is true,  causes the point of the pen to move back to the start
  /// of the current sub-path. It tries to draw a straight line from the
  /// current point to the start. If the shape has already been closed or has
  /// only one point, nothing to do.
  #[inline]
  pub fn end_path(&mut self, close: bool) { self.lyon_builder.end(close); }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, to: Point) -> &mut Self {
    self.lyon_builder.line_to(to.to_untyped());
    self
  }

  /// Adds a cubic Bezier curve to the current path.
  #[inline]
  pub fn bezier_curve_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
    self
      .lyon_builder
      .cubic_bezier_to(ctrl1.to_untyped(), ctrl2.to_untyped(), to.to_untyped());
  }

  /// Adds a quadratic Bézier curve to the current path.
  #[inline]
  pub fn quadratic_curve_to(&mut self, ctrl: Point, to: Point) {
    self
      .lyon_builder
      .quadratic_bezier_to(ctrl.to_untyped(), to.to_untyped());
  }

  /// adds a circular arc to the current sub-path, using the given control
  /// points and radius. The arc is automatically connected to the path's latest
  /// point with a straight line, if necessary for the specified
  pub fn arc_to(&mut self, center: Point, radius: f32, start_angle: Angle, end_angle: Angle) {
    let sweep_angle = end_angle - start_angle;
    let arc = Arc {
      start_angle,
      sweep_angle,
      radii: (radius, radius).into(),
      center: center.to_untyped(),
      x_rotation: Angle::zero(),
    };
    arc.for_each_quadratic_bezier(&mut |curve| {
      self
        .lyon_builder
        .quadratic_bezier_to(curve.ctrl, curve.to);
    });
  }

  /// The ellipse_to() method creates an elliptical arc centered at `center`
  /// with the `radius`. The path starts at startAngle and ends at endAngle, and
  /// travels in the direction given by anticlockwise (defaulting to
  /// clockwise).
  pub fn ellipse_to(
    &mut self, center: Point, radius: Vector, start_angle: Angle, end_angle: Angle,
  ) {
    let sweep_angle = end_angle - start_angle;
    let arc = Arc {
      start_angle,
      sweep_angle,
      radii: radius.to_untyped(),
      center: center.to_untyped(),
      x_rotation: Angle::zero(),
    };
    arc.for_each_quadratic_bezier(&mut |curve| {
      self
        .lyon_builder
        .quadratic_bezier_to(curve.ctrl, curve.to);
    });
  }

  #[inline]
  pub fn segment(&mut self, from: Point, to: Point) -> &mut Self {
    self
      .lyon_builder
      .add_line_segment(&LineSegment { from: from.to_untyped(), to: to.to_untyped() });
    self
  }

  /// Adds a sub-path containing an ellipse.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  ///
  /// # Parameters
  /// * `center` - The center point of the ellipse
  /// * `radius` - The radius vector (x and y radii) of the ellipse
  /// * `rotation` - The rotation angle of the ellipse in radians
  /// * `is_positive` - If true, adds the ellipse with positive winding (normal
  ///   fill). If false, adds the ellipse with negative winding (can be used to
  ///   exclude area).
  #[inline]
  pub fn ellipse(&mut self, center: Point, radius: Vector, rotation: f32, is_positive: bool) {
    let winding = if is_positive { Winding::Positive } else { Winding::Negative };
    self.lyon_builder.add_ellipse(
      center.to_untyped(),
      radius.to_untyped(),
      Angle::radians(rotation),
      winding,
    );
  }

  /// Adds a sub-path containing a rectangle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  ///
  /// # Parameters
  /// * `rect` - The rectangle to add to the path
  /// * `is_positive` - If true, adds the rectangle with positive winding
  ///   (normal fill). If false, adds the rectangle with negative winding (can
  ///   be used to exclude area).
  #[inline]
  pub fn rect(&mut self, rect: &Rect, is_positive: bool) -> &mut Self {
    let winding = if is_positive { Winding::Positive } else { Winding::Negative };
    self
      .lyon_builder
      .add_rectangle(&rect.to_box2d().to_untyped(), winding);
    self
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  ///
  /// # Parameters
  /// * `center` - The center point of the circle
  /// * `radius` - The radius of the circle
  /// * `is_positive` - If true, adds the circle with positive winding (normal
  ///   fill). If false, adds the circle with negative winding (can be used to
  ///   exclude area).
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32, is_positive: bool) -> &mut Self {
    let winding = if is_positive { Winding::Positive } else { Winding::Negative };
    self
      .lyon_builder
      .add_circle(center.to_untyped(), radius, winding);
    self
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  ///
  /// # Parameters
  /// * `rect` - The rectangle to add to the path
  /// * `radius` - The corner radius for rounded rectangle
  /// * `is_positive` - If true, adds the rectangle with positive winding
  ///   (normal fill). If false, adds the rectangle with negative winding (can
  ///   be used to exclude area).
  #[inline]
  pub fn rect_round(&mut self, rect: &Rect, radius: &Radius, is_positive: bool) -> &mut Self {
    let radius: &BorderRadii = unsafe { std::mem::transmute(radius) };
    let winding = if is_positive { Winding::Positive } else { Winding::Negative };
    self
      .lyon_builder
      .add_rounded_rectangle(&rect.to_box2d().cast_unit(), radius, winding);
    self
  }

  /// Construct a path from the current state of the builder.
  #[inline]
  pub fn build(self) -> Path {
    // todo: we can store an anti-aliasing flag for the path.
    self.lyon_builder.build().into()
  }

  /// Construct a path from the current state of the builder, and use the given
  /// bounds as the bounds of the path.
  ///
  /// Caller must ensure that the bounds are correct.
  pub fn build_with_bounds(self, bounds: Rect) -> Path {
    let path = self.lyon_builder.build();
    Path::new(path, bounds)
  }
}
