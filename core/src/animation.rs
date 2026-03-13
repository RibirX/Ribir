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
mod keyframes;
pub use keyframes::*;

use crate::{state::StateWatcher, window::WindowId};

///  Trait to describe how to control the animation.
pub trait Animation {
  /// Start the animation.
  fn run(&self);
  /// Stop the animation if it is running, otherwise do nothing.
  fn stop(&self);
  /// Check if the animation is running.
  fn is_running(&self) -> bool;
  /// Observe running-state changes.
  fn running_watcher(&self) -> Box<dyn StateWatcher<Value = bool>>;
  /// Initialize the target window for animations that need one.
  ///
  /// If an animation is created outside of a valid `BuildCtx`, use this
  /// function to explicitly initialize the window.
  fn init_window(&self, _window_id: WindowId) {}
  /// Clone the animation as a trait object.
  fn dyn_clone(&self) -> Box<dyn Animation>;
}

impl Clone for Box<dyn Animation> {
  fn clone(&self) -> Self { self.dyn_clone() }
}
