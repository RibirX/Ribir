//! This module defines the constant variables of the material theme that are
//! frequently used.
//!
//! See: https://m3.material.io

// The motion variables
// See https://m3.material.io/styles/motion/overview
pub mod easing {
  use ribir_core::animation::easing::CubicBezierEasing;
  pub use ribir_core::animation::easing::LINEAR;

  // The Emphasized easing set is recommended for most transitions to capture the
  // style of M3.
  pub const EMPHASIZED: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
  pub const EMPHASIZED_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 0.8, 0.15);
  pub const EMPHASIZED_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0.05, 0.7, 0.1, 1.);

  // The Standard easing set can be used for small utility focused transitions
  // that need to be quick.
  pub const STANDARD: CubicBezierEasing = CubicBezierEasing::new(0.2, 0., 0., 1.);
  pub const STANDARD_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.3, 0., 1., 1.);
  pub const STANDARD_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0., 0., 0., 1.);

  pub const LEGACY: CubicBezierEasing = CubicBezierEasing::new(0.4, 0., 0.2, 1.);
  pub const LEGACY_ACCELERATE: CubicBezierEasing = CubicBezierEasing::new(0.4, 0., 1., 1.);
  pub const LEGACY_DECELERATE: CubicBezierEasing = CubicBezierEasing::new(0., 0., 0.2, 1.);

  pub mod duration {

    use crate::Duration;
    // ## Short duration
    // These are used for small utility-focused transitions.
    pub const SHORT1: Duration = Duration::from_millis(50);
    pub const SHORT2: Duration = Duration::from_millis(100);
    pub const SHORT3: Duration = Duration::from_millis(150);
    pub const SHORT4: Duration = Duration::from_millis(200);

    // ## Medium durations
    // These are used for transitions that traverse a medium area of the screen.
    pub const MEDIUM1: Duration = Duration::from_millis(250);
    pub const MEDIUM2: Duration = Duration::from_millis(300);
    pub const MEDIUM3: Duration = Duration::from_millis(350);
    pub const MEDIUM4: Duration = Duration::from_millis(400);

    // ## Long durations
    // These durations are often paired with Emphasized easing. They're used for
    // large expressive transitions.
    pub const LONG1: Duration = Duration::from_millis(450);
    pub const LONG2: Duration = Duration::from_millis(500);
    pub const LONG3: Duration = Duration::from_millis(550);
    pub const LONG4: Duration = Duration::from_millis(600);

    // ## Extra long durations
    // Though rare, some transitions use durations above 600ms. These are usually
    // used for ambient transitions that don't involve user input.
    pub const EXR_LONG1: Duration = Duration::from_millis(700);
    pub const EXR_LONG2: Duration = Duration::from_millis(800);
    pub const EXR_LONG3: Duration = Duration::from_millis(900);
    pub const EXR_LONG4: Duration = Duration::from_millis(1000);
  }
}
