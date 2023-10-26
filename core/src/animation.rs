use std::time::Instant;

pub mod easing;
mod progress;
mod transition;
pub use easing::Easing;
pub use progress::AnimateProgress;
pub use transition::*;
mod animate;
pub use animate::*;
mod lerp;
pub use lerp::Lerp;
mod animate_state;
pub use animate_state::*;

///  Trait to describe how to control the animation.
pub trait Animation {
  /// Start the animation.
  fn run(&self);
  /// Stop the animation if it is running, otherwise do nothing.
  fn stop(&self);
  /// Advance the animation to the given time, you must start the animation
  /// before calling this method, the `at` relative to the start time.
  ///
  /// ## Panics
  ///
  /// Panics if the animation is not running.
  fn advance_to(&self, at: Instant) -> AnimateProgress;
}
