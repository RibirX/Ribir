use lyon::{geom::CubicBezierSegment, math::Point};
use ribir::prelude::Curve;

pub struct CurveBezier(CubicBezierSegment<f32>);
impl CurveBezier {
  pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> CurveBezier {
    CurveBezier(CubicBezierSegment {
      from: Point::new(0., 0.),
      ctrl1: Point::new(x1, y1),
      ctrl2: Point::new(x2, y2),
      to: Point::new(1., 1.),
    })
  }
}

impl Curve for CurveBezier {
  fn transform(&self, val: f32) -> f32 {
    if val <= 0. || val >= 1. {
      return val;
    }
    let ts = self.0.solve_t_for_x(val);
    assert!(ts.len() > 0);
    self.0.y(ts[0])
  }
}

macro_rules! CurveByBezier {
  ($func: ident => {$x1: expr, $y1: expr, $x2: expr, $y2: expr}) => {
    pub fn $func() -> Box<dyn Curve> { Box::new(CurveBezier::new($x1, $y1, $x2, $y2)) }
  };
}

// you may check the bezier's effect in https://cubic-bezier.com/
CurveByBezier!(ease => {0.25, 0.1, 0.25, 1.});
CurveByBezier!(ease_in => {0.42, 0.0, 1.0, 1.0});
CurveByBezier!(ease_out => {0., 0., 0.58, 1.});
CurveByBezier!(ease_in_out => {0.42, 0., 0.58, 1.});
CurveByBezier!(ease_in_expo => {0.95, 0.05, 0.795, 0.035});
CurveByBezier!(ease_out_expo => {0.19, 1., 0.22, 1.});

#[cfg(test)]
mod tests {
  extern crate test;
  use test::Bencher;

  use super::{Curve, CurveBezier};
  #[bench]
  fn bench_curve_bezier(b: &mut Bencher) {
    b.iter(|| {
      let sum: f32 = (0..1000)
        .map(|i| CurveBezier::new(0.3, 0.7, 0.4, 0.3).transform(i as f32 / 1001.))
        .sum();
      sum
    })
  }
}
