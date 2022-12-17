use crate::{
  prelude::*,
  ticker::{FrameMsg, FrameTicker},
};
use std::{ops::DerefMut, time::Instant};

use super::property::AnimateProperty;

#[derive(Declare)]
pub struct Animate<T, P: AnimateProperty> {
  pub transition: T,
  pub prop: P,
  pub from: P::Value,
  #[declare(skip)]
  running_info: Option<AnimateInfo<P::Value>>,
  #[declare(skip, default = ctx.wnd_ctx().frame_ticker.clone())]
  frame_ticker: FrameTicker,
}

pub struct AnimateInfo<V> {
  from: V,
  to: V,
  start_at: Instant,
  last_progress: AnimateProgress,
  // Determines if lerp value in current frame.
  already_lerp: bool,
  _tick_msg_guard: Option<SubscriptionGuard<MutRc<SingleSubscription>>>,
}

impl<'a, T: Roc, P: AnimateProperty> StateRef<'a, Animate<T, P>>
where
  Animate<T, P>: 'static,
{
  pub fn run(&mut self) {
    let new_to = self.prop.get();
    // if animate is running, animate start from current value.
    let Animate { prop, running_info, .. } = self.deref_mut();
    if let Some(AnimateInfo { from, to, last_progress, .. }) = running_info {
      *from = prop.calc_lerp_value(from, to, last_progress.value());
      *to = new_to;
    } else {
      let animate = self.clone_stateful();
      let ticker = self.frame_ticker.frame_tick_stream();
      let guard = ticker
        .subscribe(move |msg| match msg {
          FrameMsg::NewFrame(_) => {}
          FrameMsg::LayoutReady(time) => {
            let p = animate.shallow_ref().lerp(time);
            if matches!(p, AnimateProgress::Finish) {
              animate.silent_ref().stop();
            }
          }
          // use silent_ref because the state of animate change, bu no need to effect the framework.
          FrameMsg::Finish(_) => animate.silent_ref().frame_finished(),
        })
        .unsubscribe_when_dropped();

      self.running_info = Some(AnimateInfo {
        from: self.from.clone(),
        to: new_to,
        start_at: Instant::now(),
        last_progress: AnimateProgress::Dismissed,
        _tick_msg_guard: Some(guard),
        already_lerp: false,
      });
    }
  }
}

impl<T: Roc, P: AnimateProperty> Animate<T, P> {
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

    let prop = &mut self.prop;
    match progress {
      AnimateProgress::Between(rate) => {
        // the state may change during animate.
        *to = prop.get();
        let value = prop.calc_lerp_value(from, to, rate);
        prop.shallow_set(value);
      }
      AnimateProgress::Dismissed => prop.set(from.clone()),
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
      self.prop.set(info.to.clone())
    }
    info.already_lerp = false;
  }

  pub fn stop(&mut self) { self.running_info.take(); }

  #[inline]
  pub fn is_running(&self) -> bool { self.running_info.is_some() }
}
