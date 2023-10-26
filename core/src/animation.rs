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
mod stagger;
pub use stagger::Stagger;

///  Trait to describe how to control the animation.
pub trait Animation {
  /// Start the animation.
  fn run(&self);
  /// Stop the animation if it is running, otherwise do nothing.
  fn stop(&self);
  /// Check if the animation is running.
  fn is_running(&self) -> bool;
  /// clone the animation.
  fn box_clone(&self) -> Box<dyn Animation>;
}
