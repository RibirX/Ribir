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
  #[declare(skip)]
  running_info: Option<AnimateInfo<R>>,
  #[declare(skip)]
  handler: Option<AnimateHandler>,
}

#[derive(Clone)]
pub struct AnimateInfo<S> {
  from: Option<S>,
  to: Option<S>,
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

    let from = info.from.get_or_insert_with(|| self.state.init_value());
    let to = info.to.get_or_insert_with(|| self.state.finial_value());

    if let AnimateProgress::Between(rate) = progress {
      let animate_state = from.lerp(to, rate);
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
      let to = info.to.clone().expect(
        "Try to finished an animate frame which not running, 
        / ensure called `lerp` before this method.",
      );
      self.state.update(to)
    }
  }
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
    // if animate is running, animate start from current value.
    let from = self
      .running_info
      .take()
      .and_then(|info| Some(info.from?.lerp(&info.to?, info.last_progress.value())));

    self.running_info = Some(AnimateInfo {
      from,
      to: None,
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
