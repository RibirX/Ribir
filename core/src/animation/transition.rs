use crate::prelude::*;

use crate::prelude::BuildCtx;

use super::easing::Easing;
use std::time::Duration;

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare, Clone, Debug, PartialEq)]
pub struct Transition<E> {
  #[declare(default)]
  pub delay: Option<Duration>,
  pub duration: Duration,
  pub easing: E,
  #[declare(default)]
  pub repeat: Repeat,
}

/// Calc the rate of change over time.
pub trait Roc {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;
}

impl<E: Easing> Roc for Transition<E> {
  fn rate_of_change(&self, mut dur: Duration) -> AnimateProgress {
    let delay = self.delay.unwrap_or_default();
    let repeat = self.repeat.repeat_cnt();
    if dur < self.delay.unwrap_or_default() {
      AnimateProgress::Dismissed
    } else if !self.repeat.is_infinite() && dur > delay + self.duration * repeat {
      AnimateProgress::Finish
    } else {
      dur -= delay;
      let time_rate = dur.as_secs_f32() / self.duration.as_secs_f32();
      let p = self.easing.easing(time_rate - time_rate.floor());
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
