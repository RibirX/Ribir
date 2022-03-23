mod progress_state;
mod repeat_mode;
use std::time::Duration;

use crate::prelude::*;
pub use progress_state::ProgressState;
pub use repeat_mode::RepeatMode;
use rxrust::ops::box_it::LocalBoxOp;

/// the ctrl handle return by TickerRunningCtrl.listen
/// after dispose is call, the call_back would'n be call again
pub trait TickerRunningHandle {
  fn dispose(&mut self);
}

pub trait TickerRunningCtrl {
  fn state(&self) -> ProgressState;
  fn reverse(&mut self);
  fn run(&mut self);
  fn pause(&mut self);
  fn is_run(&self) -> bool;
  fn is_complete(&self) -> bool;
  fn restart(&mut self, run: bool);

  /// the call_back will be call every ticker frame when running
  fn listen(&mut self, call_back: Box<dyn FnMut(ProgressState)>) -> Box<dyn TickerRunningHandle>;
}

/// you can listen the ticker signal by the TickerCtrl, the ticker will be stop
/// after the TickerCtrl dropped
pub trait TickerAnimationCtrl: TickerRunningCtrl {
  fn with_repeat(self: Box<Self>, mode: RepeatMode) -> Box<dyn TickerAnimationCtrl>;
  fn with_round(self: Box<Self>) -> Box<dyn TickerAnimationCtrl>;
  fn span_secs(&self) -> f32;
}

/// A controller for an animation. AnimationCtrl produces values that range from
/// 0.0 to 1.0
pub trait AnimationCtrl {
  /// return the current progress, we control the progress to change the
  /// animation.
  fn state(&self) -> ProgressState;

  /// the value follow the progress changed, the animation subject the value to
  /// change.
  fn value(&self) -> f32;

  /// from subject animation can observe the value when progress change
  fn subject(&mut self) -> LocalBoxOp<'static, f32, ()>;

  fn step(&mut self, step: f32);

  fn update_to(&mut self, state: ProgressState);
}

/// from TickerProvider you can get the TickerCtrl. The application will trigger
/// the TickerCtrl every drawframe
pub trait TickerProvider {
  /// trigger the TickerCtrl tick.
  fn trigger(&mut self) -> bool;

  fn ticker_ctrl(&mut self, duration: Duration) -> Box<dyn TickerAnimationCtrl>;
}

/// AnimateState is the bridge of animate and widgets states. It tell animate
/// where it starts and ends, and how to write back the progress.

pub trait AnimateState {
  type Value;

  /// When a animate trigger, where it the state starts.
  fn state_init_value(&self) -> Self::Value;

  /// When a animate trigger, where it the state ends.
  fn state_final_value(&self) -> Self::Value;

  /// Write back the state.
  fn write_state(&mut self, v: Self::Value);
}

#[derive(Debug, Clone, Copy)]
pub struct ClosureAnimateState<I, F, W> {
  pub state_init: I,
  pub state_final: F,
  pub state_writer: W,
}

pub struct ValueAnimateState<V, W: FnMut(V)> {
  pub init_value: Option<V>,
  pub final_value: Option<V>,
  pub value_writer: W,
}

impl<I, F, W, V> AnimateState for ClosureAnimateState<I, F, W>
where
  I: Fn() -> V,
  F: Fn() -> V,
  W: FnMut(V),
{
  type Value = V;
  #[inline]
  fn state_init_value(&self) -> V { (self.state_init)() }

  #[inline]
  fn state_final_value(&self) -> V { (self.state_final)() }

  #[inline]
  fn write_state(&mut self, v: V) { (self.state_writer)(v) }
}

impl<V, W> AnimateState for ValueAnimateState<V, W>
where
  W: FnMut(V),
  V: Clone,
{
  type Value = V;

  #[inline]
  fn state_init_value(&self) -> Self::Value {
    self.init_value.clone().expect("init_value is not init.")
  }
  #[inline]
  fn state_final_value(&self) -> Self::Value {
    self.final_value.clone().expect("final_value is not init.")
  }
  #[inline]
  fn write_state(&mut self, v: Self::Value) { (self.value_writer)(v) }
}

/// Transition describe how the state change form init to final smoothly.
#[derive(Debug, Clone, Copy, Declare)]
pub struct Transition {
  /// delay how long to start.
  #[declare(strip_option)]
  pub delay: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Copy)]
pub struct Animate<S> {
  pub transition: Transition,
  pub from: S,
}

impl<S> Animate<S>
where
  S: AnimateState,
{
  pub fn register(self, _: &mut BuildCtx) -> Self {
    todo!("register animate and hold the handle");
  }

  pub fn start(&mut self) { todo!() }
}
