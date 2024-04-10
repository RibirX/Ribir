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
#[derive(Clone)]
pub struct CubicBezierEasing(CubicBezierSegment<f32>);

/// Animate at a Quadratic Bézier curve. Limit x value between [0., 1.], so
/// x-axis same as time rate (t == x ), y-axis use as for the rate of change.
///
/// Construct `CubicBezierEasing` with two control pointer, the curve always
/// start from (0., 0.) to (1., 1.).
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticBezierEasing(QuadraticBezierSegment<f32>);

/// Animates at an even speed
#[derive(Clone)]
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
    assert!((0. ..=1.).contains(&time_rate));
    self.0.y(time_rate)
  }
}

pub enum StepsJump {
  /// Denotes a left-continuous function, so that the first jump happens when
  /// the animation begins;
  JumpStart,

  /// Denotes a right-continuous function, so that the last jump happens when
  /// the animation ends;
  JumpEnd,

  /// There is no jump on either end. Instead, holding at both the 0% mark and
  /// the 100% mark, each for 1/n of the duration.
  JumpNone,

  /// Includes pauses at both the 0% and 100% marks, effectively adding a step
  /// during the animation iteration.
  JumpBoth,
}

/// Displays an animation iteration along n stops along the transition,
/// displaying each stop for equal lengths of time. For example, if n is 5,
/// there are 5 steps. Whether the animation holds temporarily at 0%, 20%, 40%,
/// 60% and 80%, on the 20%, 40%, 60%, 80% and 100%, or makes 5 stops between
/// the 0% and 100% along the animation, or makes 5 stops including the 0% and
/// 100% marks (on the 0%, 25%, 50%, 75%, and 100%) depends on which of the
/// following jump terms is used
pub fn steps(step_cnt: u32, jump: StepsJump) -> Steps {
  let time_step = 1. / step_cnt as f32;
  match jump {
    StepsJump::JumpStart => Steps { start: time_step, step: time_step, time_step },
    StepsJump::JumpEnd => Steps { start: 0., step: time_step, time_step },
    StepsJump::JumpBoth => {
      Steps { start: 1. / (step_cnt + 1) as f32, step: 1. / (step_cnt + 1) as f32, time_step }
    }
    StepsJump::JumpNone => Steps { start: 0., step: 1. / (step_cnt - 1) as f32, time_step },
  }
}
#[derive(Clone)]
pub struct Steps {
  start: f32,
  step: f32,
  time_step: f32,
}
impl Easing for Steps {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 {
    (self.start + (time_rate / self.time_step).floor() * self.step).min(1.)
  }
}

#[derive(Clone)]
pub struct StepEnd(pub f32);
impl Easing for StepEnd {
  #[inline]
  fn easing(&self, time_rate: f32) -> f32 { ((time_rate / self.0).ceil() * self.0).min(1.) }
}
