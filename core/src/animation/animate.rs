use tracing::warn;

use crate::{
  prelude::*,
  ticker::FrameMsg,
  window::{WindowFlags, WindowId},
};

#[declare(simple)]
pub struct Animate<S: AnimateState + 'static> {
  #[declare(custom, default = Self::default_transition())]
  pub transition: Box<dyn Transition>,
  #[declare(strict)]
  pub state: S,
  pub from: S::Value,
  #[declare(skip)]
  running_info: Option<AnimateInfo<S::Value>>,
  #[declare(custom, default = Self::default_window_id())]
  pub window_id: Option<WindowId>,
}

impl<S: AnimateState> AnimateDeclarer<S> {
  pub fn with_transition(&mut self, transition: impl Transition + 'static) -> &mut Self {
    self.transition = Some(Box::new(transition));
    self
  }

  pub fn default_transition() -> Box<dyn Transition> {
    Box::new(EasingTransition { easing: easing::LINEAR, duration: Duration::from_millis(300) })
  }

  pub fn default_window_id() -> Option<WindowId> {
    BuildCtx::try_get().map(|ctx| ctx.window().id())
  }

  pub fn with_window_id(&mut self, window_id: WindowId) -> &mut Self {
    self.window_id = Some(Some(window_id));
    self
  }
}

pub(crate) struct AnimateInfo<V> {
  from: V,
  to: V,
  window_id: WindowId,
  start_at: Instant,
  last_progress: AnimateProgress,
  pending_first_sample: bool,
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

    let Some(window_id) = this.window_id else {
      warn!("Animate.run skipped: window_id is not configured.");
      return;
    };
    let Some(wnd) = AppCtx::get_window(window_id) else { return };

    if !wnd.flags().contains(WindowFlags::ANIMATIONS) {
      return;
    }
    let new_to = this.state.get();

    if let Some(AnimateInfo { from, to, last_progress, start_at, .. }) = &mut this.running_info {
      *from = this
        .state
        .calc_lerp_value(from, to, last_progress.value());
      *to = new_to;
      *last_progress = AnimateProgress::Dismissed;
      *start_at = Instant::now();
      // Restart already has a visually committed `from` sampled from the
      // in-flight animation, so we should continue toward the new target on
      // the very next frame instead of inserting an extra held-from frame.
      let info = this.running_info.as_mut().unwrap();
      info.pending_first_sample = false;
      info.already_lerp = false;
    } else {
      drop(animate_ref);

      let animate = self.clone_writer();
      let this = &mut *self.write();
      let tick_handle = wnd
        .frame_ticker
        .clone()
        .subscribe(move |msg| {
          match msg {
            FrameMsg::BeforeLayout(time) | FrameMsg::LayoutReady(time) => {
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
                animate.stop();
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
        pending_first_sample: true,
        _tick_msg_guard: Some(Box::new((tick_handle, state_handle))),
        window_id,
        already_lerp: false,
      });

      wnd.inc_running_animate();
    }
  }

  fn is_running(&self) -> bool { self.read().is_running() }

  fn running_watcher(&self) -> Box<dyn StateWatcher<Value = bool>> {
    Box::new(self.part_watcher(|a| PartRef::from_value(a.is_running())))
  }

  fn init_window(&self, window_id: WindowId) { self.write().window_id = Some(window_id) }

  fn window_id(&self) -> Option<WindowId> { self.read().window_id() }

  fn stop(&self) {
    let mut animate_ref = self.write();
    let this = &mut *animate_ref;
    if this.is_running() {
      this.dec_running_animate_if_needed();
      this.running_info.take();
    }
  }

  fn dyn_clone(&self) -> Box<dyn Animation> { Box::new(self.clone_writer()) }
}

impl<S> Animate<S>
where
  S: AnimateState + 'static,
{
  pub fn interpolated_value(&mut self) -> S::Value {
    if let Some(AnimateInfo { from, to, last_progress, .. }) = self.running_info.as_mut() {
      self
        .state
        .calc_lerp_value(from, to, last_progress.value())
    } else {
      self.state.get()
    }
  }

  /// Initialize the target window for animations that need one.
  ///
  /// If an animation is created outside of a valid `BuildCtx`, use this
  /// function to explicitly initialize the window.
  pub fn init_window(&mut self, window_id: WindowId) -> &mut Self {
    self.window_id = Some(window_id);
    self
  }

  pub fn clear_window_id(&mut self) -> &mut Self {
    self.window_id = None;
    self
  }

  pub fn window_id(&self) -> Option<WindowId> { self.window_id }

  pub fn is_running(&self) -> bool { self.running_info.is_some() }

  /// Advance the animation by the given duration from its start time.
  /// This is useful for testing to manually control animation progress.
  ///
  /// ## Panics
  ///
  /// Panics if the animation is not running.
  pub fn advance_by(&mut self, elapsed: Duration) -> AnimateProgress {
    let info = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");
    let at = info.start_at + elapsed;
    self.advance_to(at)
  }

  /// Advance the animation to the given time, you must start the animation
  /// before calling this method, the `at` relative to the start time.
  ///
  /// ## Panics
  ///
  /// Panics if the animation is not running.
  pub(crate) fn advance_to(&mut self, at: Instant) -> AnimateProgress {
    let info = self
      .running_info
      .as_mut()
      .expect("This animation is not running.");

    if info.already_lerp {
      return info.last_progress;
    }

    if info.pending_first_sample {
      info.pending_first_sample = false;
      info.start_at = at;

      if self.transition.duration().is_zero() {
        info.last_progress = AnimateProgress::Finish;
        self.state.set_animating(info.to.clone());
        info.already_lerp = true;
        return AnimateProgress::Finish;
      }

      info.last_progress = AnimateProgress::Dismissed;
      self.state.set_animating(info.from.clone());
      info.already_lerp = true;
      return AnimateProgress::Dismissed;
    }

    let elapsed = at - info.start_at;
    let progress = self.transition.rate_of_change(elapsed);

    let v = match progress {
      AnimateProgress::Between(rate) => self
        .state
        .calc_lerp_value(&info.from, &info.to, rate),
      AnimateProgress::Dismissed => info.from.clone(),
      AnimateProgress::Finish => info.to.clone(),
    };
    self.state.set_animating(v);

    info.last_progress = progress;
    info.already_lerp = true;

    progress
  }
}

impl<S> Animate<S>
where
  S: AnimateState,
{
  fn running_window_id(&self) -> Option<WindowId> {
    self
      .running_info
      .as_ref()
      .map(|info| info.window_id)
  }

  fn dec_running_animate_if_needed(&self) {
    if let Some(window_id) = self.running_window_id()
      && let Some(wnd) = AppCtx::get_window(window_id)
    {
      wnd.dec_running_animate();
    }
  }
}

impl<P> Drop for Animate<P>
where
  P: AnimateState,
{
  fn drop(&mut self) {
    if self.running_info.is_some() {
      self.dec_running_animate_if_needed();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{context::AppCtx, reset_test_env, test_helper::TestWindow, window::WindowFlags};

  #[derive(Declare)]
  struct ValueRecorder {
    value: Stateful<f32>,
    frames: Stateful<Vec<f32>>,
  }

  impl Render for ValueRecorder {
    fn measure(&self, clamp: BoxClamp, _ctx: &mut MeasureCtx) -> Size { clamp.min }

    fn paint(&self, _ctx: &mut PaintingCtx) { self.frames.write().push(*self.value.read()); }
  }

  #[test]
  fn fix_animate_circular_mut_borrow() {
    reset_test_env!();

    let w = fn_widget! {
      let animate = @Animate {
        transition: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::ZERO,
        },
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
        },
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

  #[test]
  fn stop_notifies_running_watcher() {
    reset_test_env!();

    let events = Stateful::new(vec![]);
    let c_events = events.clone_reader();

    let w = fn_widget! {
      let animate = @Animate {
        transition: EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(100),
        },
        state: Stateful::new(1.),
        from: 0.,
      };
      let mounted_animate = animate.clone_writer();
      let layout_animate = animate.clone_writer();
      let running = animate.running_watcher();
      let running_reader = running.clone_boxed_watcher();
      let sub = running.raw_modifies().subscribe(move |_| {
        $write(events).push(*running_reader.read());
      });

      @Void {
        on_mounted: move |_| mounted_animate.run(),
        on_performed_layout: move |_| layout_animate.stop(),
        on_disposed: move |_| sub.unsubscribe(),
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    AppCtx::run_until_stalled();

    assert!(
      c_events.read().contains(&false),
      "stopping an animate should notify running watchers with `false`"
    );
  }

  #[test]
  fn initial_run_sampling_is_framework_only_modify() {
    reset_test_env!();

    let wnd =
      TestWindow::new(fn_widget! { @Void {} }, Size::new(10., 10.), WindowFlags::ANIMATIONS);
    let state = Stateful::new(1.0_f32);
    let data_hits = Stateful::new(0usize);
    let data_hits_reader = data_hits.clone_reader();

    let _data_sub = state
      .modifies()
      .subscribe({
        let data_hits = data_hits.clone_writer();
        move |_| *data_hits.write() += 1
      })
      .into_boxed();

    let animate = {
      let mut builder = Animate::declarer();
      builder
        .with_transition(EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        })
        .with_state(state.clone_writer())
        .with_from(0.0);
      builder.finish()
    };
    animate.init_window(wnd.id());

    animate.run();
    AppCtx::run_until_stalled();

    assert_eq!(
      *data_hits_reader.read(),
      0,
      "initial run sampling should not publish a data modify"
    );
  }

  #[test]
  fn run_does_not_mutate_state_outside_frame_lifecycle() {
    reset_test_env!();

    let wnd =
      TestWindow::new(fn_widget! { @Void {} }, Size::new(10., 10.), WindowFlags::ANIMATIONS);
    let state = Stateful::new(1.0_f32);

    let animate = {
      let mut builder = Animate::declarer();
      builder
        .with_transition(EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        })
        .with_state(state.clone_writer())
        .with_from(0.0);
      builder.finish()
    };
    animate.init_window(wnd.id());

    animate.run();

    assert_eq!(
      *state.read(),
      1.0,
      "Animate.run should not overwrite state before a frame lifecycle callback samples it"
    );
  }

  #[test]
  fn second_run_restarts_from_current_interpolated_value() {
    reset_test_env!();

    let state = Stateful::new(1.0_f32);
    let frames = Stateful::new(Vec::new());
    let frames_reader = frames.clone_reader();
    let state_in_widget = state.clone_writer();

    let animate = {
      let mut builder = Animate::declarer();
      builder
        .with_transition(EasingTransition {
          easing: easing::LINEAR,
          duration: Duration::from_millis(200),
        })
        .with_state(state.clone_writer())
        .with_from(0.0);
      builder.finish()
    };

    let wnd = TestWindow::new(
      fn_widget! {
        @ValueRecorder {
          value: state_in_widget.clone_writer(),
          frames: frames.clone_writer(),
        }
      },
      Size::new(10., 10.),
      WindowFlags::ANIMATIONS,
    );
    animate.init_window(wnd.id());

    animate.run();
    wnd.draw_frame();

    // Manually advance the animation to 80ms (40% progress, value ~0.4),
    // without relying on real time.
    animate
      .shallow()
      .advance_by(Duration::from_millis(80));
    wnd.draw_frame();
    let mid = *frames_reader
      .read()
      .last()
      .expect("animated value should be painted before restart");
    assert!(
      (mid - 0.4).abs() < 0.01,
      "first animation should be at 40% progress (value ~0.4), got {mid}"
    );

    *state.write() = 0.2;
    animate.run();
    assert_eq!(
      *state.read(),
      0.2,
      "restarting should not mutate state until the next frame samples the animated value"
    );

    wnd.draw_frame();
    let restarted = *frames_reader
      .read()
      .last()
      .expect("restarted animation should paint a sampled value");

    assert!(
      (restarted - mid).abs() < 0.01,
      "second run should restart from current interpolated value, mid={mid}, restarted={restarted}"
    );
  }
}
