use crate::{prelude::*, ticker::FrameMsg, window::WindowId};
use std::time::Instant;

#[derive(Declare)]
pub struct Animate<S>
where
  S: AnimateState + 'static,
{
  #[declare(strict, default = transitions::LINEAR.of(ctx!()))]
  pub transition: Box<dyn Roc>,
  #[declare(strict)]
  pub state: S,
  pub from: S::Value,
  #[declare(skip)]
  running_info: Option<AnimateInfo<S::Value>>,
  #[declare(skip, default = ctx!().window().id())]
  window_id: WindowId,
}

pub(crate) struct AnimateInfo<V> {
  from: V,
  to: V,
  start_at: Instant,
  last_progress: AnimateProgress,
  // Determines if lerp value in current frame.
  already_lerp: bool,
  _tick_msg_guard: Option<SubscriptionGuard<BoxSubscription<'static>>>,
}

pub trait AnimateRun<S>: StateWriter<Value = Animate<S>>
where
  S: AnimateState + 'static,
{
  fn run(&self) {
    let mut animate_ref = self.write();
    let this = &mut *animate_ref;
    let wnd_id = this.window_id;
    let new_to = this.state.get();

    if let Some(AnimateInfo { from, to, last_progress, .. }) = &mut this.running_info {
      *from = this.state.calc_lerp_value(from, to, last_progress.value());
      *to = new_to;
    } else if let Some(wnd) = AppCtx::get_window(wnd_id) {
      drop(animate_ref);

      let animate = self.clone_writer();
      let ticker = wnd.frame_ticker.frame_tick_stream();
      let unsub = ticker.subscribe(move |msg| {
        match msg {
          FrameMsg::NewFrame(time) => {
            let p = animate.read().running_info.as_ref().unwrap().last_progress;
            // Stop the animate at the next frame of animate finished, to ensure draw the
            // last frame of the animate.
            if matches!(p, AnimateProgress::Finish) {
              let wnd = AppCtx::get_window(wnd_id).unwrap();
              let animate = animate.clone_writer();
              wnd
                .frame_spawn(async move { animate.silent().stop() })
                .unwrap();
            } else {
              animate.shallow().lerp_by_instant(time);
            }
          }
          FrameMsg::LayoutReady(_) => {}
          // use silent_ref because the state of animate change, bu no need to effect the framework.
          FrameMsg::Finish(_) => {
            let animate = &mut *animate.silent();
            let info = animate.running_info.as_mut().unwrap();
            animate.state.set(info.to.clone());
            info.already_lerp = false;
          }
        }
      });
      let guard = BoxSubscription::new(unsub).unsubscribe_when_dropped();
      let animate = &mut *self.write();
      animate.running_info = Some(AnimateInfo {
        from: animate.from.clone(),
        to: new_to,
        start_at: Instant::now(),
        last_progress: AnimateProgress::Dismissed,
        _tick_msg_guard: Some(guard),
        already_lerp: false,
      });
      wnd.inc_running_animate();
    }
  }
}

impl<S, T> AnimateRun<S> for T
where
  S: AnimateState + 'static,
  T: StateWriter<Value = Animate<S>>,
{
}

impl<S> Animate<S>
where
  S: AnimateState + 'static,
{
  fn lerp_by_instant(&mut self, now: Instant) -> AnimateProgress {
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
        let value = self.state.calc_lerp_value(from, to, rate);
        // the state may change during animate.
        *to = self.state.get();
        self.state.set(value);
      }
      AnimateProgress::Dismissed => self.state.set(from.clone()),
      AnimateProgress::Finish => {}
    }

    *last_progress = progress;
    *already_lerp = true;

    progress
  }

  pub fn stop(&mut self) {
    if self.is_running() {
      if let Some(wnd) = AppCtx::get_window(self.window_id) {
        wnd.dec_running_animate();
        self.running_info.take();
      }
    }
  }

  #[inline]
  pub fn is_running(&self) -> bool { self.running_info.is_some() }
}

impl<P> Drop for Animate<P>
where
  P: AnimateState + 'static,
{
  fn drop(&mut self) {
    if self.is_running() {
      if let Some(wnd) = AppCtx::get_window(self.window_id).filter(|_| self.is_running()) {
        wnd.dec_running_animate();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{animation::easing, state::Stateful, test_helper::TestWindow};
  use std::time::Duration;

  #[test]
  fn fix_animate_circular_mut_borrow() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = fn_widget! {
      let animate = @Animate {
        transition: Transition {
          easing: easing::LINEAR,
          duration: Duration::ZERO,
        }.box_it(),
        state: Stateful::new(1.),
        from: 0.,
      };
      animate.run();
      @Void {}
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
  }
}
