pub mod easing;
mod progress;
mod repeat_mode;
mod state;
mod transition;

use std::{
  ops::{Add, Mul, Sub},
  time::Instant,
};

use crate::prelude::*;
pub use easing::Easing;
pub use progress::AnimateProgress;
pub use repeat_mode::RepeatMode;
pub use state::*;
pub use transition::*;

#[derive(Declare)]
pub struct Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
{
  transition: T,
  #[declare(rename = "from")]
  state: AnimateState<I, F, W>,
  /// Store the running information of this animation.  
  #[declare(default)]
  running_info: Option<AnimateInfo<R>>,
}

#[derive(Clone)]
pub struct AnimateInfo<S> {
  from: S,
  to: S,
  start_at: Instant,
  last_progress: AnimateProgress,
}

pub trait AnimationCtrl {
  fn start(&mut self, at: Instant);
  fn lerp_by(&mut self, now: Instant) -> AnimateProgress;
  fn frame_finished(&mut self);
}

impl<T, I, F, W, R> AnimationCtrl for Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
  T: Tween,
  R: Sub<R, Output = R> + Add<R, Output = R> + Mul<f32, Output = R> + Clone,
{
  fn start(&mut self, start_at: Instant) {
    assert!(
      self.running_info.is_none(),
      "Try to start an animation which already running."
    );
    self.running_info = Some(AnimateInfo {
      from: self.state.init_value(),
      to: self.state.finial_value(),
      start_at,
      last_progress: AnimateProgress::Dismissed,
    });
  }

  fn lerp_by(&mut self, now: Instant) -> AnimateProgress {
    let info = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");
    let elapsed = now - info.start_at;
    let progress = self.transition.tween(elapsed);

    if let AnimateProgress::Between(rate) = self.transition.tween(elapsed) {
      let animate_state = info.from.clone() + (info.to.clone() - info.from.clone()) * rate;
      self.state.update(animate_state);
    }
    info.last_progress = progress;

    progress
  }

  fn frame_finished(&mut self) {
    let info = self
      .running_info
      .clone()
      .expect("This animation is not running.");

    if matches!(info.last_progress, AnimateProgress::Between(_)) {
      self.state.update(info.to.clone())
    }
  }
}
