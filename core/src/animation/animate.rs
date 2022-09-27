use crate::{
  prelude::*,
  ticker::{FrameMsg, FrameTicker},
};
use std::time::Instant;

#[derive(Declare)]
pub struct Animate<T, I, F, W, R, L>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R) + 'static,
  L: FnMut(&R, &R, f32) -> R,
{
  pub transition: T,
  #[declare(rename = from)]
  state: AnimateState<I, F, W>,
  /// function calc the linearly lerp value by rate, three arguments are
  /// `from` `to` and `rate`, specify `lerp_fn` when the animate state not
  /// implement `Lerp` trait or you want to specify a custom lerp function.
  lerp_fn: L,
  #[declare(skip)]
  running_info: Option<AnimateInfo<R>>,
  #[declare(skip, default = ctx.app_ctx().borrow().frame_ticker.clone())]
  frame_ticker: FrameTicker,
}

pub struct AnimateInfo<S> {
  from: S,
  to: S,
  start_at: Instant,
  last_progress: AnimateProgress,
  // Determines if lerp value in current frame.
  already_lerp: bool,
  _tick_msg_guard: Option<SubscriptionGuard<MutRc<SingleSubscription>>>,
}

impl<'a, T, I, F, W, R, L> StateRef<'a, Animate<T, I, F, W, R, L>>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R),
  T: Roc,
  R: Clone,
  L: FnMut(&R, &R, f32) -> R,
  Animate<T, I, F, W, R, L>: 'static,
{
  pub fn run(&mut self) {
    let new_to = self.state.finial_value();
    let Animate { lerp_fn, running_info, .. } = &mut **self;
    // if animate is running, animate start from current value.
    if let Some(info) = running_info {
      let AnimateInfo { from, to, last_progress, .. } = info;
      *from = (lerp_fn)(from, to, last_progress.value());
      *to = new_to;
    } else {
      let animate = self.clone_stateful();
      let ticker = self.frame_ticker.frame_tick_stream();
      let guard = ticker
        .subscribe(move |msg| match msg {
          FrameMsg::NewFrame(_) => {}
          FrameMsg::LayoutReady(time) => {
            let mut inner = animate.raw_ref();
            let p = inner.lerp(time);
            if matches!(p, AnimateProgress::Finish) {
              inner.stop();
            }
          }
          // use silent_ref because the state of animate change, bu no need to effect the framework.
          FrameMsg::Finish(_) => animate.silent_ref().frame_finished(),
        })
        .unsubscribe_when_dropped();
      let from = self.state.init_value();
      self.running_info = Some(AnimateInfo {
        from,
        to: new_to,
        start_at: Instant::now(),
        last_progress: AnimateProgress::Dismissed,
        _tick_msg_guard: Some(guard),
        already_lerp: false,
      });
    }
  }
}

impl<T, I, F, W, R, L> Animate<T, I, F, W, R, L>
where
  I: Fn() -> R,
  F: Fn() -> R,
  W: FnMut(R),
  T: Roc,
  R: Clone,
  L: FnMut(&R, &R, f32) -> R,
{
  fn lerp(&mut self, now: Instant) -> AnimateProgress {
    let AnimateInfo {
      from,
      to,
      start_at,
      last_progress,
      already_lerp,
      ..
    } = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");

    if *already_lerp {
      return *last_progress;
    }

    let elapsed = now - *start_at;
    let progress = self.transition.rate_of_change(elapsed);

    match progress {
      AnimateProgress::Between(rate) => {
        // the state may change during animate.
        *to = self.state.finial_value();
        let animate_state = (self.lerp_fn)(from, to, rate);
        self.state.update(animate_state);
      }
      AnimateProgress::Dismissed => self.state.update(from.clone()),
      AnimateProgress::Finish => {}
    }

    *last_progress = progress;
    *already_lerp = true;

    progress
  }

  fn frame_finished(&mut self) {
    let info = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");

    if !matches!(info.last_progress, AnimateProgress::Finish) {
      self.state.update(info.to.clone())
    }
    info.already_lerp = false;
  }

  pub fn stop(&mut self) { self.running_info.take(); }

  #[inline]
  pub fn is_running(&self) -> bool { self.running_info.is_some() }
}
