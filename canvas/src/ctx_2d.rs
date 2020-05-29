use lyon::{
  math::point,
  path::{path::Builder, Path},
};

pub struct Ctx2D {
  path: Builder,
}

impl Ctx2D {
  pub fn new() -> Self {
    let path = Path::builder();
    Ctx2D { path }
  }

  #[inline]
  pub fn get_path(self) -> Path { self.path.build() }

  /// Starts a new path by emptying the list of sub-paths.
  /// Call this method when you want to create a new path.
  #[inline]
  pub fn begin_path(&mut self, x: f32, y: f32) -> &mut Self {
    self.path.begin(point(x, y));
    self
  }

  /// Causes the point of the pen to move back to the start of the current
  /// sub-path. It tries to draw a straight line from the current point to the
  /// start. If the shape has already been closed or has only one point, this
  #[inline]
  pub fn close_path(&mut self) { self.path.close(); }

  /// Connects the last point in the current sub-path to the specified (x, y)
  /// coordinates with a straight line.
  #[inline]
  pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
    self.path.line_to(point(x, y));
    self
  }

  /// Adds a cubic Bézier curve to the current path.
  #[inline]
  pub fn bezier_curve_to(
    &mut self,
    cp1x: f32,
    cp1y: f32,
    cp2x: f32,
    cp2y: f32,
    x: f32,
    y: f32,
  ) -> &mut Self {
    self.path.cubic_bezier_to(
      point(cp1x, cp1y),
      point(cp2x, cp2y),
      point(x, y),
    );
    self
  }

  /// Adds a quadratic Bézier curve to the current path.
  #[inline]
  pub fn quadratic_curve_to(
    &mut self,
    cpx: f32,
    cpy: f32,
    x: f32,
    y: f32,
  ) -> &mut Self {
    self.path.quadratic_bezier_to(point(cpx, cpy), point(x, y));
    self
  }

  /// Adds a circular arc to the current path.
  pub fn arc(
    &mut self,
    x: f32,
    y: f32,
    radius: f32,
    startAngle: f32,
    endAngle: f32,
    anticlockwise: Option<f32>,
  ) -> &mut Self {
    unimplemented!();
  }

  /// Adds an arc to the current path with the given control points and radius,
  /// connected to the previous point by a straight line.
  pub fn arc_to(
    &mut self,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    radius: f32,
  ) -> &mut Self {
    unimplemented!();
  }

  /// Adds an elliptical arc to the current path.
  pub fn ellipse(
    &mut self,
    x: f32,
    y: f32,
    radiusX: f32,
    radiusY: f32,
    rotation: f32,
    startAngle: f32,
    endAngle: f32,
    anticlockwise: Option<f32>,
  ) -> &mut Self {
    unimplemented!();
  }

  /// Creates a path for a rectangle at position (x, y) with a size that is
  /// determined by width and height.
  pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) -> &mut Self {
    let tl_pt = point(x, y);
    let tr_pt = point(x + width, y);
    let bl_pt = point(x, y + height);
    let br_pt = point(x + width, y + height);
    self.path.begin(tl_pt);
    self.path.line_to(tr_pt);
    self.path.line_to(br_pt);
    self.path.line_to(bl_pt);
    self.close_path();
    self
  }
}
