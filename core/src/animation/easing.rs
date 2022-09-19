use lyon_geom::{CubicBezierSegment, QuadraticBezierSegment};

/// Specify the rate of change of the rate of over time.
pub trait Easing {
  fn easing(&self, time_rate: f32) -> f32;
}

/// Animate at a Cubic Bézier curve. Limit x value between [0., 1.], so x-axis
/// same as time rate (t == x ), y-axis use as for the rate of change.
///
/// Construct `CubicBezierEasing` with two control pointer, the curve always
/// start from (0., 0.) to (1., 1.).
pub struct CubicBezierEasing(CubicBezierSegment<f32>);

/// Animate at a Quadratic Bézier curve. Limit x value between [0., 1.], so
/// x-axis same as time rate (t == x ), y-axis use as for the rate of change.
///
/// Construct `CubicBezierEasing` with two control pointer, the curve always
/// start from (0., 0.) to (1., 1.).
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticBezierEasing(QuadraticBezierSegment<f32>);

/// Animates at an even speed
pub struct LinearEasing;

// Some const easing cubic bezier provide.
// reference: https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function

/// Increases in velocity towards the middle of the animation, slowing back down
/// at the end.
pub const EASE: CubicBezierEasing = CubicBezierEasing::new(0.25, 0.1, 0.25, 1.0);

///  Animates at an even speed
pub const LINEAR: LinearEasing = LinearEasing;

/// Starts off slowly, with the speed of the transition of the animating
/// property increasing until complete.
pub const EASE_IN: QuadraticBezierEasing = QuadraticBezierEasing::new(0.42, 0.);

/// Starts quickly, slowing down the animation continues.
pub const EASE_OUT: QuadraticBezierEasing = QuadraticBezierEasing::new(0.58, 1.);

/// With the animating properties slowly transitioning, speeding up, and then
/// slowing down again.
pub const EASE_IN_OUT: CubicBezierEasing = CubicBezierEasing::new(0.42, 0., 0.58, 1.);

impl CubicBezierEasing {
  /// Construct cubic bezier by two control point,
  ///
  /// #Panic
  ///
  /// The values of `x1` and `x2` must be in the range of 0 to 1, panic
  /// otherwise.
  pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
    use lyon_geom::Point as LPoint;
    Self(CubicBezierSegment {
      from: LPoint::new(0., 0.),
      ctrl1: LPoint::new(x1, y1),
      ctrl2: LPoint::new(x2, y2),
      to: LPoint::new(1., 1.),
    })
  }
}

/// Construct cubic bezier by two control point,
///
/// #Panic
///
/// The values of `x` must be in the range of 0 to 1, panic
/// otherwise.
impl QuadraticBezierEasing {
  pub const fn new(x: f32, y: f32) -> Self {
    use lyon_geom::Point as LPoint;
    Self(QuadraticBezierSegment {
      from: LPoint::new(0., 0.),
      ctrl: LPoint::new(x, y),
      to: LPoint::new(1., 1.),
    })
  }
}

impl Easing for LinearEasing {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 { time_rate }
}

impl Easing for QuadraticBezierEasing {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 { self.0.y(time_rate) }
}

impl Easing for CubicBezierEasing {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 {
    assert!(0. <= time_rate && time_rate <= 1.);
    self.0.y(time_rate)
  }
}

pub struct Throld(pub f32);
impl Easing for Throld {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 { if time_rate < self.0 { 0. } else { 1. } }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use super::*;
  use test::Bencher;

  #[bench]
  fn bench_curve_bezier(b: &mut Bencher) {
    b.iter(|| {
      let sum: f32 = (0..1000)
        .map(|i| CubicBezierEasing::new(0.3, 0.7, 0.4, 0.3).easing(i as f32 / 1001.))
        .sum();
      sum
    })
  }
}
