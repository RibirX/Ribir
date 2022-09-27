use crate::prelude::*;

use crate::prelude::BuildCtx;

use super::easing::Easing;
use std::time::Duration;

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare, Clone, Debug, PartialEq)]
pub struct Transition<E> {
  pub duration: Duration,
  pub easing: E,
}

/// Calc the rate of change over time.
pub trait Roc {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;

  fn duration(&self) -> Duration;
}

pub trait RocWithRepeat<R: Roc> {
  fn repeat(self, r: Repeat) -> TransitionWithRepeat<R>;
}

pub trait RocWithDelay<R: Roc> {
  fn delay(self, delay: Duration) -> TransitionWithDelay<R>;
}

impl<E: Easing> Roc for Transition<E> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    if dur > self.duration() {
      AnimateProgress::Finish
    } else {
      let time_rate = dur.as_secs_f32() / self.duration.as_secs_f32();
      let p = self.easing.easing(time_rate);
      AnimateProgress::Between(p)
    }
  }

  fn duration(&self) -> Duration { self.duration }
}

impl<E: Easing> Roc for Stateful<Transition<E>> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    self.state_ref().rate_of_change(dur)
  }

  fn duration(&self) -> Duration { self.state_ref().duration }
}

/// Transition with delay
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionWithDelay<R: Roc> {
  delay: Duration,
  inner: R,
}

impl<R: Roc> Roc for TransitionWithDelay<R> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    if dur < self.delay {
      AnimateProgress::Dismissed
    } else {
      self.inner.rate_of_change(dur - self.delay)
    }
  }

  fn duration(&self) -> Duration { self.inner.duration() + self.delay }
}

impl<R: Roc> Roc for Stateful<TransitionWithDelay<R>> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    self.state_ref().rate_of_change(dur)
  }
  #[inline]
  fn duration(&self) -> Duration { self.state_ref().duration() }
}

// transition with repeat
pub struct TransitionWithRepeat<R: Roc> {
  repeat: Repeat,
  inner: R,
}

impl<R: Roc> Roc for TransitionWithRepeat<R> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    if !self.repeat.is_infinite() && dur > self.duration() {
      AnimateProgress::Finish
    } else {
      // use the f32 to calc instead of the Duration,
      // because the insufficient precision of floating number may result the zero be
      // negatived, which will cause panic when use Duration directly.
      let round_time = self.inner.duration().as_secs_f32();
      let dur_f32 = dur.as_secs_f32();
      let round = (dur_f32 / round_time).floor();
      let p = self.inner.rate_of_change(Duration::from_secs_f32(
        (dur_f32 - round_time * round).max(0.),
      ));
      AnimateProgress::Between(p.value())
    }
  }

  fn duration(&self) -> Duration {
    let repeat = self.repeat.repeat_cnt();
    self.inner.duration() * repeat
  }
}

impl<R: Roc> Roc for Stateful<TransitionWithRepeat<R>> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    self.state_ref().rate_of_change(dur)
  }
  #[inline]
  fn duration(&self) -> Duration { self.state_ref().duration() }
}

impl<R: Roc> RocWithDelay<R> for R {
  fn delay(self, delay: Duration) -> TransitionWithDelay<R> {
    TransitionWithDelay::<R> { delay, inner: self }
  }
}

impl<R: Roc> RocWithRepeat<R> for R {
  fn repeat(self, repeat: Repeat) -> TransitionWithRepeat<R> {
    TransitionWithRepeat::<R> { repeat, inner: self }
  }
}
