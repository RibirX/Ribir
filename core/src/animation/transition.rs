use crate::prelude::*;

use crate::prelude::BuildCtx;

use super::{easing::Easing, RepeatMode};
use std::time::Duration;

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare, Clone, Copy)]
pub struct Transition<E> {
  #[declare(default)]
  pub delay: Option<Duration>,
  pub duration: Duration,
  #[declare(default = "RepeatMode::None")]
  pub repeat: RepeatMode,
  pub easing: E,
}

/// Calc the rate of change over time.
pub trait Tween {
  /// Calc the rate of change of the duration from animation start.
  fn tween(&self, dur: Duration) -> AnimationProgress;
}

impl<E: Easing> Tween for Transition<E> {
  fn tween(&self, dur: Duration) -> AnimationProgress {
    let delay = self.delay.unwrap_or_default();
    if dur < self.delay.unwrap_or_default() {
      AnimationProgress::Dismissed
    } else if dur > delay + self.duration {
      AnimationProgress::Finish
    } else {
      let time_rate = dur.as_secs_f32() / self.duration.as_secs_f32();
      let p = self.easing.easing(time_rate);
      AnimationProgress::Between(p)
    }
  }
}
