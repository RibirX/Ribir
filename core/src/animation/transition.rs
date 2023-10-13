use super::easing::Easing;
use crate::prelude::{BuildCtx, *};
use std::{ops::Deref, rc::Rc, time::Duration};

/// Transition use rate to describe how the state change form init to final
/// smoothly.
#[derive(Declare, Clone, Debug, PartialEq)]
pub struct Transition<E: 'static> {
  #[declare(default)]
  pub delay: Option<Duration>,
  pub duration: Duration,
  #[declare(strict)]
  pub easing: E,
  #[declare(default)]
  pub repeat: Option<f32>,
}

/// Trait help to transition the state.
pub trait TransitionState: Sized + 'static {
  /// Use an animate to transition the state after it modified.
  fn transition<T: Roc + 'static>(self, transition: T, ctx: &BuildCtx) -> Writer<Animate<T, Self>>
  where
    Self: AnimateState,
    <Self::State as StateReader>::Value: Clone,
  {
    let state = self.state().clone_writer();
    let from = state.read().clone();
    let mut animate: State<Animate<T, Self>> = Animate::declare_builder()
      .transition(transition)
      .from(from)
      .state(self)
      .build_declare(ctx);

    let c_animate = animate.clone_writer();
    let init_value = observable::of(state.read().clone());
    state
      .modifies()
      .map(move |_| state.read().clone())
      .merge(init_value)
      .pairwise()
      .subscribe(move |(old, _)| {
        animate.write().from = old;
        animate.run();
      });
    c_animate
  }

  /// Transition the state with a lerp function.
  fn transition_with<T, F>(
    self,
    transition: T,
    lerp_fn: F,
    ctx: &BuildCtx,
  ) -> Writer<Animate<T, LerpFnState<Self, F>>>
  where
    Self: StateWriter,
    Self::Value: Clone,
    F: FnMut(&Self::Value, &Self::Value, f32) -> Self::Value + 'static,
    T: Roc + 'static,
  {
    let animate_state = LerpFnState::new(self, lerp_fn);
    animate_state.transition(transition, ctx)
  }
}

/// Calc the rate of change over time.
pub trait Roc {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;
}

impl<E: Easing> Roc for Transition<E> {
  fn rate_of_change(&self, mut run_dur: Duration) -> AnimateProgress {
    if self.duration.as_secs_f32().abs() < f32::EPSILON {
      return AnimateProgress::Finish;
    }

    if let Some(delay) = self.delay {
      if run_dur < delay {
        return AnimateProgress::Dismissed;
      } else {
        run_dur -= delay;
      }
    }

    let repeat = self.repeat.unwrap_or(1.);

    let rounds = run_dur.as_secs_f32() / self.duration.as_secs_f32();
    if rounds > repeat {
      return AnimateProgress::Finish;
    }

    let time_rate = run_dur.as_secs_f32() / self.duration.as_secs_f32() - rounds.floor();
    let p = self.easing.easing(time_rate);
    AnimateProgress::Between(p)
  }
}

impl<T: StateReader> Roc for T
where
  T::Value: Roc,
{
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { self.read().rate_of_change(dur) }
}

impl<S: StateWriter + 'static> TransitionState for S {}

impl<V, S, F> TransitionState for LerpFnState<S, F>
where
  S: StateWriter<Value = V> + 'static,
  F: FnMut(&V, &V, f32) -> V + 'static,
{
}

impl Roc for Box<dyn Roc> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { self.deref().rate_of_change(dur) }
}

impl<T: Roc> Roc for Rc<T> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { self.deref().rate_of_change(dur) }
}
