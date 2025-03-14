use ribir_algo::Sc;

use crate::prelude::*;

/// Transition use `Easing` trait to calc the rate of change over time.
#[derive(Clone, Debug)]
pub struct EasingTransition<E: 'static> {
  pub duration: Duration,
  pub easing: E,
}

/// Transition will apply after the delay duration.
#[derive(Clone)]
pub struct DelayTransition<T> {
  pub delay: Duration,
  pub transition: T,
}

/// Transition will apply with repeat times.
#[derive(Clone)]
pub struct RepeatTransition<T> {
  pub repeat: f32,
  pub transition: T,
}

/// Transition is a trait to help you calc the rate of change over time.
pub trait Transition {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;

  /// Return the duration of the animation from start to finish.
  fn duration(&self) -> Duration;

  /// Transition will apply with repeat times
  fn repeat(self, repeat: f32) -> RepeatTransition<Self>
  where
    Self: Sized,
  {
    RepeatTransition { repeat, transition: self }
  }

  /// Transition will apply after the delay duration
  fn delay(self, delay: Duration) -> DelayTransition<Self>
  where
    Self: Sized,
  {
    DelayTransition { delay, transition: self }
  }

  fn box_it(self) -> Box<dyn Transition>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}

// todo: remove it after `TransitionTheme` is removed
pub trait RocBoxClone: Transition {
  fn box_clone(&self) -> Box<dyn Transition>;
}

impl<T: Transition + Clone + 'static> RocBoxClone for T {
  fn box_clone(&self) -> Box<dyn Transition> { self.clone().box_it() }
}

impl<T: Transition> Transition for DelayTransition<T> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    if dur < self.delay {
      return AnimateProgress::Dismissed;
    }
    self.transition.rate_of_change(dur - self.delay)
  }

  fn duration(&self) -> Duration { self.delay + self.transition.duration() }
}

impl<T: Transition> Transition for RepeatTransition<T> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    let repeat = self.repeat;
    let duration = self.transition.duration();
    let rounds = dur.as_secs_f32() / duration.as_secs_f32();
    if rounds > repeat {
      return AnimateProgress::Finish;
    }
    let rate = match self
      .transition
      .rate_of_change(duration.mul_f32(rounds.fract()))
    {
      AnimateProgress::Dismissed => 0.,
      AnimateProgress::Finish => 1.,
      AnimateProgress::Between(p) => p,
    };
    AnimateProgress::Between(rate)
  }

  fn duration(&self) -> Duration {
    let duration = self.transition.duration();
    let repeat = self.repeat;
    Duration::from_secs_f32(duration.as_secs_f32() * repeat)
  }
}

impl Transition for Box<dyn Transition> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { (**self).rate_of_change(dur) }

  fn duration(&self) -> Duration { (**self).duration() }
}

impl<T: Transition> Transition for Sc<T> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { (**self).rate_of_change(dur) }

  fn duration(&self) -> Duration { (**self).duration() }
}

impl<E: Easing> Transition for EasingTransition<E> {
  fn rate_of_change(&self, run_dur: Duration) -> AnimateProgress {
    if run_dur > self.duration {
      return AnimateProgress::Finish;
    }
    let time_rate = run_dur.as_secs_f32() / self.duration.as_secs_f32();
    let p = self.easing.easing(time_rate);
    AnimateProgress::Between(p)
  }

  fn duration(&self) -> Duration { self.duration }
}
