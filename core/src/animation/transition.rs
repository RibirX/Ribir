use super::easing::Easing;
use crate::prelude::{BuildCtx, *};
use std::time::Duration;

/// Transition use rate to describe how the state change form init to final
/// smoothly.
#[derive(Clone, Debug)]
pub struct Transition<E: 'static> {
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

/// Trait help to transition the state.
pub trait TransitionState: Sized + 'static {
  /// Use an animate to transition the state after it modified.
  fn transition(self, transition: Box<dyn Roc>, ctx: &BuildCtx) -> Writer<Animate<Self>>
  where
    Self: AnimateState,
  {
    let state = self.clone_setter();
    let animate: State<Animate<Self>> = Animate::declare_builder()
      .transition(transition)
      .from(self.get())
      .state(self)
      .build_declare(ctx);

    let c_animate = animate.clone_writer();
    let init_value = observable::of(state.get());
    state
      .animate_state_modifies()
      .map(move |_| state.get())
      .merge(init_value)
      .pairwise()
      .subscribe(move |(old, _)| {
        animate.write().from = old;
        animate.run();
      });
    c_animate
  }
}

impl<S: AnimateState + 'static> TransitionState for S {}

/// Transition the state with a lerp function.
pub trait TransitionWithFn: AnimateStateSetter + Sized {
  fn transition_with<F>(
    self,
    transition: Box<dyn Roc>,
    lerp_fn: F,
    ctx: &BuildCtx,
  ) -> Writer<Animate<LerpFnState<Self, F>>>
  where
    F: FnMut(&Self::Value, &Self::Value, f32) -> Self::Value + 'static,
  {
    let animate_state = LerpFnState::new(self, lerp_fn);
    animate_state.transition(transition, ctx)
  }
}

impl<S> TransitionWithFn for S where S: AnimateStateSetter {}

/// Calc the rate of change over time.
pub trait Roc {
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

  fn box_it(self) -> Box<dyn Roc>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}

pub trait RocBoxClone: Roc {
  fn box_clone(&self) -> Box<dyn Roc>;
}

impl<T: Roc + Clone + 'static> RocBoxClone for T {
  fn box_clone(&self) -> Box<dyn Roc> { self.clone().box_it() }
}

impl<T: Roc> Roc for DelayTransition<T> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    if dur < self.delay {
      return AnimateProgress::Dismissed;
    }
    self.transition.rate_of_change(dur - self.delay)
  }

  fn duration(&self) -> Duration { self.delay + self.transition.duration() }
}

impl<T: Roc> Roc for RepeatTransition<T> {
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

impl Roc for Box<dyn Roc> {
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { (**self).rate_of_change(dur) }

  fn duration(&self) -> Duration { (**self).duration() }
}

impl<E: Easing> Roc for Transition<E> {
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
