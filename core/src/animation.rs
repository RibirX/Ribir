mod animation_ctrl;
mod progress_state;
mod repeat_mode;
mod tween;
use std::{marker::PhantomData, time::Duration};

use crate::prelude::*;
pub use animation_ctrl::*;
pub use progress_state::ProgressState;
pub use repeat_mode::RepeatMode;
use rxrust::ops::box_it::LocalCloneBoxOp;
pub use tween::Tween;

/// the ctrl handle return by TickerRunningCtrl.listen
/// after dispose is call, the call_back would'n be call again
pub trait TickerRunningHandle {
  fn dispose(&mut self);
}

pub trait TickerRunningCtrl {
  fn state(&self) -> ProgressState;
  fn reverse(&mut self);
  fn start(&mut self);
  fn pause(&mut self);
  fn is_run(&self) -> bool;
  fn is_complete(&self) -> bool;
  fn restart(&mut self, run: bool);
  fn force_done(&mut self);

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
  fn subject(&mut self) -> LocalCloneBoxOp<'static, f32, ()>;

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

// Transform the Region from [0.0 - 1.0] to [0.0 - 1.0]
// animation use the curve map to implement ease effect
pub trait Curve {
  fn transform(&self, t: f32) -> f32;
}

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare)]
pub struct Transition {
  /// delay how long to start.
  // #[declare(strip_option)]
  // pub delay: Option<std::time::Duration>,
  pub duration: std::time::Duration,

  pub repeat: RepeatMode,

  pub ease: Option<fn() -> Box<dyn Curve + 'static>>,
}

pub struct Animate<V, S, R>
where
  S: FnMut(V, V) -> R + 'static + Clone,
  R: FnMut(f32) + 'static + Clone,
{
  update_animate: S,
  tick: Box<dyn TickerRunningCtrl>,
  observable: LocalCloneBoxOp<'static, f32, ()>,
  guard: Option<SubscriptionGuard<Box<dyn SubscriptionLike>>>,
  __: PhantomData<R>,
  ___: PhantomData<V>,
}

impl<V, S, R> Animate<V, S, R>
where
  S: FnMut(V, V) -> R + 'static + Clone,
  R: FnMut(f32) + 'static + Clone,
  V: std::fmt::Debug,
{
  pub fn new(update_animate: S, transition: &Transition, ctx: &mut BuildCtx) -> Self {
    let mut tick = ctx
      .ticker_ctrl(transition.duration)
      .unwrap()
      .with_repeat(transition.repeat);

    let mut animation = new_animation_ctrl(transition.ease.as_ref().map(|f| f()));

    let observable = animation.subject();

    tick.listen(Box::new(move |p| animation.update_to(p)));

    Animate {
      update_animate,
      tick,
      observable,
      guard: None,
      __: PhantomData,
      ___: PhantomData,
    }
  }

  pub fn default() -> Self { todo!() }

  pub fn restart(&mut self, init_v: V, final_v: V) {
    let mut update = (self.update_animate)(init_v, final_v);

    self.guard = Some(
      self
        .observable
        .clone()
        .subscribe(move |p| {
          update(p);
        })
        .unsubscribe_when_dropped(),
    );

    self.tick.restart(true);
  }

  pub fn cancel(&mut self) { self.tick.force_done(); }
}

impl TransitionBuilder {
  pub fn build_without_ctx(self) -> Transition {
    self
      .duration
      .expect(&format!("Required field Transition::duration` not set"));

    // Safety: we know build `Transition` will never read the ctx.
    #[allow(invalid_value)]
    let mut uninit_ctx: &mut BuildCtx = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    let t = self.build(&mut uninit_ctx);
    std::mem::forget(uninit_ctx);
    t
  }
}
