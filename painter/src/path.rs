use crate::{Angle, PathStyle, Point, Rect, Vector};
use lyon_tessellation::path::Path as LyonPath;
pub use lyon_tessellation::{
  path::{
    builder::BorderRadii,
    geom::{Arc, LineSegment},
    path::Builder as LyonBuilder,
    traits::PathBuilder,
    Winding,
  },
  StrokeOptions,
};
use serde::{Deserialize, Serialize};

/// Path widget describe a shape, build the shape from [`Builder`]!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
  pub path: LyonPath,
  pub style: PathStyle,
}

/// The radius of each corner of a rounded rectangle.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct Radius(BorderRadii);

#[derive(Default)]
pub struct Builder(LyonBuilder);

impl Path {
  #[inline]
  pub fn builder() -> Builder { Builder::default() }

  #[inline]
  pub fn box_rect(&self) -> Rect {
    // todo: path_style effect box rect
    lyon_algorithms::aabb::bounding_rect(self.path.iter()).cast_unit()
  }
}

impl Builder {
  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, at: Point) -> &mut Self {
    self.0.begin(at.to_untyped());
    self
  }

  /// Tell the builder the sub-path is finished.
  /// if `close` is true,  causes the point of the pen to move back to the start
  /// of the current sub-path. It tries to draw a straight line from the
  /// current point to the start. If the shape has already been closed or has
  /// only one point, nothing to do.
  #[inline]
  pub fn end_path(&mut self, close: bool) { self.0.end(close); }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, to: Point) -> &mut Self {
    self.0.line_to(to.to_untyped());
    self
  }

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

  #[inline]
  pub fn segment(&mut self, from: Point, to: Point) -> &mut Self {
    self.0.add_line_segment(&LineSegment {
      from: from.to_untyped(),
      to: to.to_untyped(),
    });
    self
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
  pub fn rect(&mut self, rect: &Rect) -> &mut Self {
    self.0.add_rectangle(&rect.to_untyped(), Winding::Positive);
    self
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32) -> &mut Self {
    self
      .0
      .add_circle(center.to_untyped(), radius, Winding::Positive);
    self
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  /// #[inline]
  pub fn rect_round(&mut self, rect: &Rect, radius: &Radius) -> &mut Self {
    // Safety, just a unit type convert, it's same type.
    let rect = unsafe { std::mem::transmute(rect) };
    self
      .0
      .add_rounded_rectangle(rect, radius, Winding::Positive);
    self
  }

  /// Build a stroke path with `width` size, and `style`.
  #[inline]
  pub fn stroke(self, options: StrokeOptions) -> Path {
    Path {
      path: self.0.build(),
      style: PathStyle::Stroke(options),
    }
  }

  /// Build a fill path, witch should fill with `style`
  #[inline]
  pub fn fill(self) -> Path {
    Path {
      path: self.0.build(),
      style: PathStyle::Fill,
    }
  }
}

impl std::ops::Deref for Radius {
  type Target = BorderRadii;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for Radius {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Radius {
  #[inline]
  pub fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> Radius {
    BorderRadii {
      top_left,
      top_right,
      bottom_left,
      bottom_right,
    }
    .into()
  }

  /// Creates a radius where all radii are radius.
  #[inline]
  pub fn all(radius: f32) -> Radius { Self::new(radius, radius, radius, radius) }

  #[inline]
  pub fn left(left: f32) -> Radius { Self::new(left, 0., left, 0.) }

  #[inline]
  pub fn right(right: f32) -> Radius { Self::new(0., right, 0., right) }

  #[inline]
  pub fn top(top: f32) -> Radius { Self::new(top, top, 0., 0.) }

  #[inline]
  pub fn bottom(bottom: f32) -> Radius { Self::new(0., 0., bottom, bottom) }

  /// Creates a horizontally symmetrical radius where the left and right sides
  /// of the rectangle have the same radii.
  #[inline]
  pub fn horizontal(left: f32, right: f32) -> Radius { Self::new(left, right, left, right) }

  ///Creates a vertically symmetric radius where the top and bottom sides of
  /// the rectangle have the same radii.
  #[inline]
  pub fn vertical(top: f32, bottom: f32) -> Radius { Self::new(top, top, bottom, bottom) }

  #[inline]
  pub fn top_left(top_left: f32) -> Self { Radius(BorderRadii { top_left, ..<_>::default() }) }

  #[inline]
  pub fn top_right(top_right: f32) -> Self { Radius(BorderRadii { top_right, ..<_>::default() }) }

  #[inline]
  pub fn bottom_left(bottom_left: f32) -> Self {
    Radius(BorderRadii { bottom_left, ..<_>::default() })
  }

  #[inline]
  pub fn bottom_right(bottom_right: f32) -> Self {
    Radius(BorderRadii { bottom_right, ..<_>::default() })
  }
}

impl From<Radius> for BorderRadii {
  #[inline]
  fn from(radius: Radius) -> Self { radius.0 }
}

impl From<BorderRadii> for Radius {
  #[inline]
  fn from(radii: BorderRadii) -> Self { Self(radii) }
}
