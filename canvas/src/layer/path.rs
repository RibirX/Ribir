use crate::*;
use lyon::{
  geom::Arc,
  path::{builder::PathBuilder as LyonBuilder, Winding},
};

pub struct PathBuilder(lyon::path::path::Builder);

#[derive(Debug, Clone)]
pub struct Path(pub(crate) lyon::path::Path);

#[derive(Debug, Default, Clone)]
pub struct BorderRadius {
  pub top_left: Vector,
  pub top_right: Vector,
  pub bottom_left: Vector,
  pub bottom_right: Vector,
}

impl PathBuilder {
  #[inline]
  pub fn new() -> Self { Default::default() }

  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, at: Point) { self.0.begin(at.to_untyped()); }

  /// Causes the point of the pen to move back to the start of the current
  /// sub-path. It tries to draw a straight line from the current point to the
  /// start. If the shape has already been closed or has only one point, this
  #[inline]
  pub fn close_path(&mut self) { self.0.close(); }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, to: Point) { self.0.line_to(to.to_untyped()); }

  /// Adds a cubic Bezier curve to the current path.
  #[inline]
  pub fn bezier_curve_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
    self
      .0
      .cubic_bezier_to(ctrl1.to_untyped(), ctrl2.to_untyped(), to.to_untyped());
  }

  /// Adds a quadratic Bézier curve to the current path.
  #[inline]
  pub fn quadratic_curve_to(&mut self, ctrl: Point, to: Point) {
    self
      .0
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
      self.0.quadratic_bezier_to(curve.ctrl, curve.to);
    });
  }

  /// The ellipse_to() method creates an elliptical arc centered at `center`
  /// with the `radius`. The path starts at startAngle and ends at endAngle, and
  /// travels in the direction given by anticlockwise (defaulting to
  /// clockwise).
  pub fn ellipse_to(
    &mut self,
    center: Point,
    radius: Vector,
    start_angle: Angle,
    end_angle: Angle,
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
      self.0.quadratic_bezier_to(curve.ctrl, curve.to);
    });
  }

  /// Adds a sub-path containing an ellipse.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn ellipse(&mut self, center: Point, radius: Vector, rotation: f32) {
    self.0.add_ellipse(
      center.to_untyped(),
      radius.to_untyped(),
      Angle::radians(rotation),
      Winding::Positive,
    );
  }

  /// Adds a sub-path containing a rectangle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn rect(&mut self, rect: &Rect) {
    self.0.add_rectangle(&rect.to_untyped(), Winding::Positive);
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32) {
    self
      .0
      .add_circle(center.to_untyped(), radius, Winding::Positive)
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  pub fn rect_round(&mut self, rect: &Rect, radius: &BorderRadius) {
    let BorderRadius {
      top_left,
      top_right,
      bottom_left,
      bottom_right,
    } = radius;

    let w = rect.width();
    let h = rect.height();
    let mut tl_x = top_left.x.abs().min(w);
    let mut tl_y = top_left.y.abs().min(h);
    let mut tr_x = top_right.x.abs().min(w);
    let mut tr_y = top_right.y.abs().min(h);
    let mut bl_x = bottom_left.x.abs().min(w);
    let mut bl_y = bottom_left.y.abs().min(h);
    let mut br_x = bottom_right.x.abs().min(w);
    let mut br_y = bottom_right.y.abs().min(h);
    if tl_x + tr_x > w {
      let shrink = (tl_x + tr_x - w) / 2.;
      tl_x -= shrink;
      tr_x -= shrink;
    }
    if bl_x + br_x > w {
      let shrink = (bl_x + br_x - w) / 2.;
      bl_x -= shrink;
      br_x -= shrink;
    }
    if tl_y + bl_y > h {
      let shrink = (tl_y + bl_y - h) / 2.;
      tl_y -= shrink;
      bl_y -= shrink;
    }
    if tr_y + br_y > h {
      let shrink = (tr_y + br_y - h) / 2.;
      tr_y -= shrink;
      br_y -= shrink;
    }

    let max = rect.max();
    if tl_x > 0. && tl_y > 0. {
      self.begin_path(Point::new(rect.min_x(), rect.min_y() + tl_y));
      let radius = Vector::new(tl_x, tl_y);
      self.ellipse_to(
        rect.min() + radius,
        radius,
        Angle::degrees(180.),
        Angle::degrees(270.),
      );
    } else {
      self.begin_path(rect.min());
    }
    if tr_x > 0. && tr_y > 0. {
      let radius = Vector::new(tr_x, tr_y);
      let center = Point::new(max.x - tr_x, rect.min_y() + tr_y);
      self.line_to(Point::new(max.x - tr_x, rect.min_y()));
      self.ellipse_to(center, radius, Angle::degrees(270.), Angle::degrees(360.));
    } else {
      self.line_to(Point::new(max.x, rect.min_y()));
    }
    if br_x > 0. && br_y > 0. {
      let radius = Vector::new(br_x, br_y);
      let center = max - radius;
      self.line_to(Point::new(max.x, max.y - br_y));
      self.ellipse_to(center, radius, Angle::degrees(0.), Angle::degrees(90.));
    } else {
      self.line_to(max);
    }

    if bl_x > 0. && bl_y > 0. {
      let radius = Vector::new(bl_x, bl_y);
      self.line_to(Point::new(rect.min_x() + bl_x, max.y));
      self.ellipse_to(
        Point::new(rect.min_x() + bl_x, max.y - bl_y),
        radius,
        Angle::degrees(90.),
        Angle::degrees(180.),
      );
    } else {
      self.line_to(Point::new(rect.min_x(), max.y));
    }

    self.close_path();
  }

  #[inline]
  pub fn build(self) -> Path { Path(self.0.build()) }
}

impl BorderRadius {
  pub fn all(radius: Vector) -> Self {
    Self {
      top_left: radius,
      top_right: radius,
      bottom_left: radius,
      bottom_right: radius,
    }
  }
}

impl Default for PathBuilder {
  fn default() -> Self { Self(lyon::path::path::Builder::new()) }
}
