use lyon_geom::CubicBezierSegment;

/// Specify the rate of change of the rate of over time.
pub trait Easing {
  fn easing(&self, time_rate: f32) -> f32;
}

/// Describe how change rate transform with time over.
/// x-axis for time rate, y-axis for the rate of change.
pub struct CubicBezier(CubicBezierSegment<f32>);

// Some const easing cubic bezier provide.
// reference: https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function

/// Increases in velocity towards the middle of the animation, slowing back down
/// at the end.
pub const EASE: CubicBezier = CubicBezier::new(0.25, 0.1, 0.25, 1.0);

///  Animates at an even speed
pub const LINEAR: CubicBezier = CubicBezier::new(0., 0., 1., 1.);

/// starts off slowly, with the speed of the transition of the animating
/// property increasing until complete.
pub const EASE_IN: CubicBezier = CubicBezier::new(0.42, 0., 1., 1.);

/// starts quickly, slowing down the animation continues.
pub const EASE_OUT: CubicBezier = CubicBezier::new(0., 0., 0.58, 1.);

/// with the animating properties slowly transitioning, speeding up, and then
/// slowing down again.
pub const EASE_IN_OUT: CubicBezier = CubicBezier::new(0.42, 0., 0.58, 1.);

impl CubicBezier {
  pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> CubicBezier {
    use lyon_geom::Point as LPoint;
    CubicBezier(CubicBezierSegment {
      from: LPoint::new(0., 0.),
      ctrl1: LPoint::new(x1, y1),
      ctrl2: LPoint::new(x2, y2),
      to: LPoint::new(1., 1.),
    })
  }
}

impl Easing for CubicBezier {
  fn easing(&self, time_rate: f32) -> f32 {
    assert!(0. <= time_rate && time_rate <= 1.);
    let ts = self.0.solve_t_for_x(time_rate);
    assert!(ts.len() > 0);
    self.0.y(ts[0])
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use test::Bencher;

  use super::{CubicBezier, Easing};
  #[bench]
  fn bench_curve_bezier(b: &mut Bencher) {
    b.iter(|| {
      let sum: f32 = (0..1000)
        .map(|i| CubicBezier::new(0.3, 0.7, 0.4, 0.3).easing(i as f32 / 1001.))
        .sum();
      sum
    })
  }
}
