use crate::prelude::{
  widget_tree::animation_store::{AnimateHandler, AnimateStore},
  *,
};
use std::time::Instant;

#[derive(Declare)]
pub struct Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
{
  pub transition: T,
  #[declare(rename = from)]
  state: AnimateState<I, F, W>,
  /// todo: declare skip fields, Store the running information of this
  /// animation.
  #[declare(default)]
  running_info: Option<AnimateInfo<R>>,
  handler: Option<AnimateHandler>,
}

#[derive(Clone)]
pub struct AnimateInfo<S> {
  from: S,
  to: S,
  start_at: Instant,
  last_progress: AnimateProgress,
}

pub trait AnimateCtrl {
  /// lerp animate value at `now`
  fn lerp(&mut self, now: Instant) -> AnimateProgress;
  /// State data should be rollback after draw.
  fn frame_finished(&mut self);
}

impl<T, I, F, W, R> AnimateCtrl for Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
  T: Roc,
  R: Lerp + Clone,
{
  fn lerp(&mut self, now: Instant) -> AnimateProgress {
    let info = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");
    let elapsed = now - info.start_at;
    let progress = self.transition.rate_of_change(elapsed);

    if let AnimateProgress::Between(rate) = progress {
      let animate_state = info.from.lerp(&info.to, rate);
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

impl<T, I, F, W, R> IntoStateful for Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
{
  #[inline]
  fn into_stateful(self) -> Stateful<Self> { Stateful::new(self) }
}

impl<T: AnimateCtrl> AnimateCtrl for Stateful<T> {
  #[inline]
  fn lerp(&mut self, now: Instant) -> AnimateProgress { self.state_ref().lerp(now) }

  #[inline]
  fn frame_finished(&mut self) { self.state_ref().frame_finished() }
}

impl<T, I, F, W, R> Animate<T, I, F, W, R>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R),
  T: Roc,
  R: Lerp + Clone,
  Self: 'static,
{
  pub fn register(this: &Stateful<Self>, ctx: &BuildCtx) {
    assert!(this.raw_ref().handler.is_none());

    let handler = AnimateStore::register(&ctx.tree.animations_store, this.clone());
    this.raw_ref().handler = Some(handler);
  }

  pub fn run(&mut self) {
    let from = if let Some(info) = self.running_info.take() {
      info.to
    } else {
      self.state.init_value()
    };

    self.running_info = Some(AnimateInfo {
      from,
      to: self.state.finial_value(),
      start_at: Instant::now(),
      last_progress: AnimateProgress::Dismissed,
    });
    self.register_handler().running_start();
  }

  pub fn stop(&mut self) {
    self.running_info.take();
    self.register_handler().stopped();
  }

  #[inline]
  pub fn is_running(&self) -> bool { self.running_info.is_some() }

  /// Unregister the animate.
  pub fn unregister(&mut self) {
    if let Some(handler) = self.handler.take() {
      handler.unregister();
    } else {
      log::warn!("Unregister an animate which not registered.")
    }
  }

  fn register_handler(&self) -> &AnimateHandler {
    self
      .handler
      .as_ref()
      .expect("Can't call on an unregister animate.")
  }
}
