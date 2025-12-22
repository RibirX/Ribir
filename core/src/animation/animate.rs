use crate::{
  prelude::*,
  ticker::FrameMsg,
  window::{WindowFlags, WindowId},
};

#[simple_declare]
pub struct Animate<S: AnimateState + 'static> {
  #[declare(custom, default = Self::default_transition())]
  pub transition: Box<dyn Transition>,
  #[declare(strict)]
  pub state: S,
  pub from: S::Value,
  #[declare(skip)]
  running_info: Option<AnimateInfo<S::Value>>,
  #[declare(skip, default = BuildCtx::get().window().id())]
  window_id: WindowId,
}

impl<S: AnimateState> AnimateDeclarer<S> {
  pub fn with_transition(&mut self, transition: impl Transition + 'static) -> &mut Self {
    self.transition = Some(Box::new(transition));
    self
  }

  pub fn default_transition() -> Box<dyn Transition> {
    Box::new(EasingTransition { easing: easing::LINEAR, duration: Duration::from_millis(300) })
  }
}

pub(crate) struct AnimateInfo<V> {
  from: V,
  to: V,
  start_at: Instant,
  last_progress: AnimateProgress,
  // Determines if lerp value in current frame.
  already_lerp: bool,
  _tick_msg_guard: Option<Box<dyn Any>>,
}

impl<S> Animation for Stateful<Animate<S>>
where
  S: AnimateState<Value: Clone> + 'static,
{
  fn run(&self) {
    let mut animate_ref = self.write();
    let this = &mut *animate_ref;

    let Some(wnd) = AppCtx::get_window(this.window_id) else { return };

    if !wnd.flags().contains(WindowFlags::ANIMATIONS) {
      return;
    }

    let new_to = this.state.get();

    if let Some(AnimateInfo { from, to, last_progress, start_at, .. }) = &mut this.running_info {
      *from = this
        .state
        .calc_lerp_value(from, to, last_progress.value());
      *to = new_to;
      *last_progress = AnimateProgress::Between(0.);
      *start_at = Instant::now();
    } else {
      drop(animate_ref);

      let animate = self.clone_writer();
      let this = &mut *self.write();
      let tick_handle = wnd
        .frame_ticker
        .clone()
        .subscribe(move |msg| {
          match msg {
            FrameMsg::BeforeLayout(time) => {
              animate.shallow().advance_to(time);
            }
            FrameMsg::Finish(_) => {
              let mut w_ref = animate.write();
              let info = w_ref.running_info.as_mut().unwrap();
              let last_progress = info.last_progress;
              let to = info.to.clone();
              info.already_lerp = false;
              w_ref.state.revert(to);
              // Forgets modifies because we only modifies the inner info.
              w_ref.forget_modifies();

              if matches!(last_progress, AnimateProgress::Finish) {
                drop(w_ref);
                let animate = animate.clone_writer();
                AppCtx::spawn_local(async move { animate.stop() });
              }
            }
            _ => {}
          }
        })
        .unsubscribe_when_dropped();

      let animate = self.clone_writer();
      let state_handle = this
        .state
        .animate_state_modifies()
        // State transition may trigger an animation run during a state modification, causing borrow
        // conflicts. To avoid this, delay the run until the next tick.
        .delay_subscription(Duration::ZERO)
        .subscribe(move |_| {
          let mut animate = animate.write();
          let v = animate.state.get();
          // if the animate state modified, we need to update the restore value.
          if let Some(info) = animate.running_info.as_mut() {
            info.to = v;
          }
          animate.forget_modifies();
        })
        .unsubscribe_when_dropped();

      this.running_info = Some(AnimateInfo {
        from: this.from.clone(),
        to: new_to,
        start_at: Instant::now(),
        last_progress: AnimateProgress::Dismissed,
        _tick_msg_guard: Some(Box::new((tick_handle, state_handle))),
        already_lerp: false,
      });

      wnd.inc_running_animate();
    }
  }

  fn is_running(&self) -> bool { self.read().is_running() }

  fn stop(&self) {
    let mut this = self.silent();
    if this.is_running()
      && let Some(wnd) = AppCtx::get_window(this.window_id)
    {
      wnd.dec_running_animate();
      this.running_info.take();
    }
  }

  fn box_clone(&self) -> Box<dyn Animation> { Box::new(self.clone_writer()) }
}

impl<S> Animate<S>
where
  S: AnimateState + 'static,
{
  pub fn is_running(&self) -> bool { self.running_info.is_some() }

  /// Advance the animation to the given time, you must start the animation
  /// before calling this method, the `at` relative to the start time.
  ///
  /// ## Panics
  ///
  /// Panics if the animation is not running.
  fn advance_to(&mut self, at: Instant) -> AnimateProgress {
    let AnimateInfo { from, to, start_at, last_progress, already_lerp, .. } = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");

    if *already_lerp {
      return *last_progress;
    }

    let elapsed = at - *start_at;
    let progress = self.transition.rate_of_change(elapsed);

    let v = match progress {
      AnimateProgress::Between(rate) => self.state.calc_lerp_value(from, to, rate),
      AnimateProgress::Dismissed => from.clone(),
      AnimateProgress::Finish => to.clone(),
    };
    self.state.set(v);

    *last_progress = progress;
    *already_lerp = true;

    progress
  }
}

impl<P> Drop for Animate<P>
where
  P: AnimateState,
{
  fn drop(&mut self) {
    if self.running_info.is_some()
      && let Some(wnd) = AppCtx::get_window(self.window_id)
    {
      wnd.dec_running_animate();
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{reset_test_env, test_helper::TestWindow};

  #[test]
  fn fix_animate_circular_mut_borrow() {
    reset_test_env!();

    let w = fn_widget! {
      let animate = @Animate {
        transition: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::ZERO,
        }.box_it(),
        state: Stateful::new(1.),
        from: 0.,
      };
      animate.run();
      @Void {}
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
  }

  #[test]
  fn fix_write_state_during_animate_running() {
    reset_test_env!();
    let state = Stateful::new(0);
    let c_state = state.clone_reader();
    let w = fn_widget! {
      let animate = @Animate {
        transition: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(1),
        }.box_it(),
        state: state.clone_writer(),
        from: 100,
      };

      animate.run();

      @Void { on_performed_layout: move |_| *$write(state) = 1 }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();

    assert_eq!(*c_state.read(), 1);
  }
}
