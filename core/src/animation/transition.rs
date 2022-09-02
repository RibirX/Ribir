use crate::prelude::*;

use crate::prelude::BuildCtx;

use super::easing::Easing;
use std::time::Duration;

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare, Clone, Copy)]
pub struct Transition<E> {
  #[declare(default)]
  pub delay: Option<Duration>,
  pub duration: Duration,
  pub easing: E,
}

/// Calc the rate of change over time.
pub trait Roc {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;
}

impl<E: Easing> Roc for Transition<E> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    let delay = self.delay.unwrap_or_default();
    if dur < self.delay.unwrap_or_default() {
      AnimateProgress::Dismissed
    } else if dur > delay + self.duration {
      AnimateProgress::Finish
    } else {
      let time_rate = dur.as_secs_f32() / self.duration.as_secs_f32();
      let p = self.easing.easing(time_rate);
      AnimateProgress::Between(p)
    }
  }
}

impl<E: Easing> Roc for Stateful<Transition<E>> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    self.state_ref().rate_of_change(dur)
  }
}
