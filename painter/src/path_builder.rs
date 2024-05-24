use lyon_algorithms::path::{
  builder::BorderRadii,
  geom::{Arc, LineSegment},
  path::Builder as LyonBuilder,
  Event, Path as LyonPath, Winding,
};
use ribir_geom::{Angle, Point, Rect, Transform, Vector};

use crate::{LineCap, LineJoin, Path, Radius, StrokeOptions};

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
  #[inline]
  pub fn ellipse(&mut self, center: Point, radius: Vector, rotation: f32) {
    self.lyon_builder.add_ellipse(
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
    self
      .lyon_builder
      .add_rectangle(&rect.to_box2d().to_untyped(), Winding::Positive);
    self
  }

  /// Adds a sub-path containing a circle.
  ///
  /// There must be no sub-path in progress when this method is called.
  /// No sub-path is in progress after the method is called.
  #[inline]
  pub fn circle(&mut self, center: Point, radius: f32) -> &mut Self {
    self
      .lyon_builder
      .add_circle(center.to_untyped(), radius, Winding::Positive);
    self
  }

  /// Creates a path for a rectangle by `rect` with `radius`.
  /// #[inline]
  pub fn rect_round(&mut self, rect: &Rect, radius: &Radius) -> &mut Self {
    let radius: &BorderRadii = unsafe { std::mem::transmute(radius) };
    self.lyon_builder.add_rounded_rectangle(
      &rect.to_box2d().cast_unit(),
      radius,
      Winding::Positive,
    );
    self
  }

  /// Return a path that strokes (outlines) the current path with the stroke
  /// options.
  #[inline]
  pub fn stroke(self, options: &StrokeOptions, ts: Option<&Transform>) -> Option<Path> {
    let path = self.lyon_builder.build();
    stroke_path(&path, options, ts).map(Into::into)
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
    Path { lyon_path: path, bounds }
  }
}

pub(crate) fn stroke_path(
  path: &LyonPath, options: &StrokeOptions, ts: Option<&Transform>,
) -> Option<LyonPath> {
  let mut builder = tiny_skia_path::PathBuilder::default();
  let resolution = ts.map_or(1., |t| {
    let t = into_tiny_transform(*t);
    tiny_skia_path::PathStroker::compute_resolution_scale(&t)
  });

  path.iter().for_each(|e| match e {
    Event::Begin { at } => builder.move_to(at.x, at.y),
    Event::Line { to, .. } => builder.line_to(to.x, to.y),
    Event::Quadratic { ctrl, to, .. } => builder.quad_to(ctrl.x, ctrl.y, to.x, to.y),
    Event::Cubic { ctrl1, ctrl2, to, .. } => {
      builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y)
    }
    Event::End { close, .. } => {
      if close {
        builder.close()
      }
    }
  });

  let path = builder
    .finish()
    .unwrap()
    .stroke(&options.clone().into(), resolution)?;

  let mut builder = LyonPath::svg_builder();
  path.segments().for_each(|seg| match seg {
    tiny_skia_path::PathSegment::MoveTo(at) => {
      builder.move_to((at.x, at.y).into());
    }
    tiny_skia_path::PathSegment::LineTo(to) => {
      builder.line_to((to.x, to.y).into());
    }
    tiny_skia_path::PathSegment::QuadTo(c, t) => {
      builder.quadratic_bezier_to((c.x, c.y).into(), (t.x, t.y).into());
    }
    tiny_skia_path::PathSegment::CubicTo(c1, c2, to) => {
      builder.cubic_bezier_to((c1.x, c1.y).into(), (c2.x, c2.y).into(), (to.x, to.y).into());
    }
    tiny_skia_path::PathSegment::Close => builder.close(),
  });
  Some(builder.build())
}

fn into_tiny_transform(ts: Transform) -> tiny_skia_path::Transform {
  let Transform { m11, m12, m21, m22, m31, m32, .. } = ts;
  tiny_skia_path::Transform { sx: m11, kx: m21, ky: m12, sy: m22, tx: m31, ty: m32 }
}

impl From<StrokeOptions> for tiny_skia_path::Stroke {
  fn from(value: StrokeOptions) -> Self {
    let StrokeOptions { width, miter_limit, line_cap, line_join } = value;
    tiny_skia_path::Stroke {
      width,
      miter_limit,
      line_cap: match line_cap {
        LineCap::Butt => tiny_skia_path::LineCap::Butt,
        LineCap::Round => tiny_skia_path::LineCap::Round,
        LineCap::Square => tiny_skia_path::LineCap::Square,
      },
      line_join: match line_join {
        LineJoin::Miter => tiny_skia_path::LineJoin::Miter,
        LineJoin::Round => tiny_skia_path::LineJoin::Round,
        LineJoin::Bevel => tiny_skia_path::LineJoin::Bevel,
        LineJoin::MiterClip => tiny_skia_path::LineJoin::MiterClip,
      },
      dash: None,
    }
  }
}
