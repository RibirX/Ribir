use std::cell::{Cell, RefCell};

use ribir_core::{
  animation::{Animation, IntoAnimation},
  prelude::*,
  window::{WindowFlags, WindowId},
};
use rxrust::subscription::{BoxedSubscription, SubscriptionGuard};
use tracing::warn;

/// A stagger animation dispatches child animations with progressively delayed
/// start times.
#[derive(Clone)]
pub struct Stagger(Rc<StaggerInner>);

struct StaggerInner {
  interval: Duration,
  window_id: Cell<Option<WindowId>>,
  animations: RefCell<Vec<StaggerItem>>,
  run_times: Cell<usize>,
  running: Stateful<bool>,
  active_run: RefCell<Option<ActiveRun>>,
}

struct StaggerItem {
  offset: Duration,
  animation: Box<dyn Animation>,
}

struct ActiveRun {
  start_at: Instant,
  next_to_run: usize,
  _timer: Option<SubscriptionGuard<TaskHandle>>,
  _watchers: Vec<SubscriptionGuard<BoxedSubscription>>,
}

impl Stagger {
  pub fn new(interval: Duration, animations: Vec<Box<dyn Animation>>) -> Self {
    let stagger = Self(Rc::new(StaggerInner {
      interval,
      window_id: Cell::new(Self::default_window_id()),
      animations: RefCell::new(vec![]),
      run_times: Cell::new(0),
      running: Stateful::new(false),
      active_run: RefCell::new(None),
    }));

    for animation in animations {
      stagger.push_animation(animation);
    }
    stagger
  }

  pub fn clone_writer(&self) -> Self { self.clone() }

  pub fn run_times(&self) -> usize { self.0.run_times.get() }

  pub fn has_ever_run(&self) -> bool { self.run_times() > 0 }

  fn default_window_id() -> Option<WindowId> { BuildCtx::try_get().map(|ctx| ctx.window().id()) }

  /// Appends an animation to the back of the stagger sequence.
  /// Only accepted while the stagger is idle.
  pub fn push_animation(&self, animation: impl IntoAnimation) -> &Self {
    self.push_animation_with(self.default_interval(), animation)
  }

  /// Appends an animation with a custom delay relative to the previous child.
  pub fn push_animation_with(&self, interval: Duration, animation: impl IntoAnimation) -> &Self {
    if self.is_running() {
      warn!("Stagger: cannot push animation while running.");
      return self;
    }

    let animation = animation.into_animation();
    if let Some(wid) = self.0.window_id.get() {
      animation.init_window(wid);
    }

    let offset = self.next_offset(interval);
    self
      .0
      .animations
      .borrow_mut()
      .push(StaggerItem { offset, animation });
    self
  }

  fn drive_run(&self, now: Instant) {
    let mut due = vec![];
    let next_at = {
      let mut active_run = self.0.active_run.borrow_mut();
      let Some(run) = active_run.as_mut() else { return };
      let animations = self.0.animations.borrow();

      let elapsed = now - run.start_at;
      while let Some(item) = animations.get(run.next_to_run) {
        if elapsed < item.offset {
          break;
        }
        due.push(item.animation.clone());
        run.next_to_run += 1;
      }

      animations
        .get(run.next_to_run)
        .map(|item| run.start_at + item.offset)
    };

    due.into_iter().for_each(|a| a.run());

    if let Ok(mut active_run) = self.0.active_run.try_borrow_mut()
      && let Some(run) = active_run.as_mut()
    {
      run._timer = next_at.map(|at| {
        let stagger = self.clone();
        Local::timer_at(at)
          .subscribe(move |_| stagger.drive_run(Instant::now()))
          .unsubscribe_when_dropped()
      });
    }

    self.sync_run_state();
  }

  fn sync_run_state(&self) {
    let mut active_run = self.0.active_run.borrow_mut();
    if let Some(run) = active_run.as_ref() {
      let animations = self.0.animations.borrow();
      let all_dispatched = run.next_to_run == animations.len();
      let any_child_running = animations
        .iter()
        .any(|item| item.animation.is_running());
      if all_dispatched && !any_child_running {
        *active_run = None;
      }
    }
    let is_running = active_run.is_some();
    if *self.0.running.read() != is_running {
      *self.0.running.write() = is_running;
    }
  }

  fn default_interval(&self) -> Duration {
    if self.0.animations.borrow().is_empty() { Duration::ZERO } else { self.0.interval }
  }

  fn next_offset(&self, interval: Duration) -> Duration {
    self
      .0
      .animations
      .borrow()
      .last()
      .map_or(interval, |item| item.offset + interval)
  }

  fn ensure_window_initialized(&self) -> Option<WindowId> {
    let animations = self.0.animations.borrow();
    let mut wid = self.0.window_id.get();

    for item in animations.iter() {
      if let Some(child_wid) = item.animation.window_id() {
        if let Some(wid) = wid {
          if wid != child_wid {
            warn!("Stagger.run skipped: child animations belong to multiple windows.");
            return None;
          }
        } else {
          wid = Some(child_wid);
        }
      }
    }

    let wid = wid.or_else(|| {
      warn!("Stagger.run skipped: window_id is not configured.");
      None
    })?;

    self.0.window_id.set(Some(wid));
    for item in animations.iter() {
      item.animation.init_window(wid);
    }
    Some(wid)
  }
}

impl Animation for Stagger {
  fn run(&self) {
    if self.is_running() {
      self.stop();
    }

    let Some(wid) = self.ensure_window_initialized() else { return };
    let Some(wnd) = AppCtx::get_window(wid) else { return };
    if !wnd.flags().contains(WindowFlags::ANIMATIONS) {
      return;
    }

    let start_at = Instant::now();
    self.0.run_times.set(self.0.run_times.get() + 1);
    if self.0.animations.borrow().is_empty() {
      return;
    }

    let run = ActiveRun {
      start_at,
      next_to_run: 0,
      _timer: None,
      _watchers: self
        .0
        .animations
        .borrow()
        .iter()
        .map(|item| {
          let stagger = self.clone();
          item
            .animation
            .running_watcher()
            .raw_modifies()
            .subscribe(move |_| stagger.sync_run_state())
            .into_boxed()
            .unsubscribe_when_dropped()
        })
        .collect(),
    };
    self.0.active_run.replace(Some(run));
    self.drive_run(start_at);
  }

  fn stop(&self) {
    if self.is_running() {
      self.0.active_run.borrow_mut().take();
      self
        .0
        .animations
        .borrow()
        .iter()
        .for_each(|i| i.animation.stop());
      self.sync_run_state();
    }
  }

  fn is_running(&self) -> bool { *self.0.running.read() }

  fn running_watcher(&self) -> Box<dyn StateWatcher<Value = bool>> {
    Box::new(self.0.running.clone_watcher())
  }

  fn init_window(&self, window_id: WindowId) {
    self.0.window_id.set(Some(window_id));
    self.ensure_window_initialized();
  }

  fn window_id(&self) -> Option<WindowId> { self.0.window_id.get() }

  fn dyn_clone(&self) -> Box<dyn Animation> { Box::new(self.clone()) }
}

#[cfg(test)]
mod tests {
  use ribir_core::{animation::Animate, reset_test_env, test_helper::*, window::WindowFlags};

  use super::*;

  fn fast_animate() -> Stateful<Animate<Stateful<f32>>> {
    let state = Stateful::new(1.0_f32);
    let mut animate = Animate::declarer();
    animate
      .with_transition(EasingTransition { duration: Duration::ZERO, easing: easing::LINEAR })
      .with_state(state)
      .with_from(0.);
    animate.finish()
  }

  fn timed_animate(ms: u64) -> Stateful<Animate<Stateful<f32>>> {
    let state = Stateful::new(1.0_f32);
    rdl! {
      Animate {
        transition: EasingTransition {
          duration: Duration::from_millis(ms),
          easing: easing::LINEAR,
        },
        state,
        from: 0.,
      }
    }
  }

  #[test]
  fn smoke_run_stop() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let stagger = Stagger::new(Duration::from_millis(100), vec![fast_animate().into_animation()]);
    stagger.init_window(wnd.id());

    stagger.run();
    assert!(stagger.is_running());
    stagger.stop();
    assert!(!stagger.is_running());
  }

  #[test]
  fn natural_completion() {
    reset_test_env!();
    let stagger = Stagger::new(Duration::ZERO, vec![]);
    let c_stagger = stagger.clone_writer();
    let wnd = TestWindow::new(
      fn_widget! {
        let animate = fast_animate();
        stagger.push_animation(animate);
        stagger.run();
        @Void {}
      },
      Size::zero(),
      WindowFlags::ANIMATIONS,
    );

    assert!(c_stagger.is_running());
    wnd.draw_frame();
    AppCtx::run_until_stalled();
    assert!(!c_stagger.is_running());
  }

  #[test]
  fn sequence_dispatch_timing() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let a1 = timed_animate(100);
    let a2 = timed_animate(100);
    let stagger =
      Stagger::new(Duration::from_millis(100), vec![a1.clone_writer().into_animation()]);
    stagger.init_window(wnd.id());
    stagger.push_animation_with(Duration::from_millis(50), a2.clone_writer());

    stagger.run();
    assert!(a1.is_running() && !a2.is_running());

    AppCtx::run_until(AppCtx::timer(Duration::from_millis(60)));
    AppCtx::run_until_stalled();
    assert!(a2.is_running());
  }

  #[test]
  fn window_id_inference() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);

    // From BuildCtx
    let captured = Rc::new(RefCell::new(None));
    let c_captured = captured.clone();
    let _ = TestWindow::new(
      fn_widget! {
        let stagger = Stagger::new(Duration::ZERO, vec![]);
        c_captured.borrow_mut().replace(stagger.clone_writer());
        @Void {}
      },
      Size::zero(),
      WindowFlags::ANIMATIONS,
    );
    assert!(
      captured
        .borrow()
        .as_ref()
        .unwrap()
        .window_id()
        .is_some()
    );

    // From children
    let a = fast_animate();
    a.init_window(wnd.id());
    let stagger = Stagger::new(Duration::ZERO, vec![a.into_animation()]);
    stagger.run();
    assert_eq!(stagger.window_id(), Some(wnd.id()));
  }

  #[test]
  fn cross_window_rejection() {
    reset_test_env!();
    let w1 = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let w2 = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);

    let a = fast_animate();
    a.init_window(w1.id());
    let stagger = Stagger::new(Duration::ZERO, vec![a.into_animation()]);
    stagger.init_window(w2.id());
    stagger.run();
    assert!(!stagger.is_running());
  }

  #[test]
  fn immutability_while_running() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let a1 = timed_animate(100);
    let a2 = timed_animate(100);
    let stagger = Stagger::new(Duration::from_millis(100), vec![a1.into_animation()]);
    stagger.init_window(wnd.id());

    stagger.run();
    stagger.push_animation(a2.clone_writer());

    AppCtx::run_until(AppCtx::timer(Duration::from_millis(200)));
    AppCtx::run_until_stalled();
    assert!(!a2.is_running());
  }

  #[test]
  fn rerun_restarts_timeline() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let a = timed_animate(100);
    let stagger = Stagger::new(Duration::ZERO, vec![]);
    stagger.init_window(wnd.id());
    stagger.push_animation_with(Duration::from_millis(100), a.clone_writer());

    stagger.run();
    AppCtx::run_until(AppCtx::timer(Duration::from_millis(50)));
    AppCtx::run_until_stalled();
    assert!(!a.is_running());

    stagger.run(); // Restart
    AppCtx::run_until(AppCtx::timer(Duration::from_millis(60)));
    AppCtx::run_until_stalled();
    assert!(!a.is_running()); // Should still be pending because 50 + 60 > 100 doesn't matter, it's a new 100ms
  }

  #[test]
  fn running_watcher_signals() {
    reset_test_env!();
    let wnd = TestWindow::new(fn_widget! { @Void {} }, Size::zero(), WindowFlags::ANIMATIONS);
    let stagger = Stagger::new(Duration::ZERO, vec![fast_animate().into_animation()]);
    stagger.init_window(wnd.id());

    let signals = Stateful::new(vec![]);
    let c_signals = signals.clone_writer();
    let watcher = stagger.running_watcher();
    let c_watcher = watcher.clone_boxed_watcher();
    let _sub = watcher.raw_modifies().subscribe(move |_| {
      c_signals.write().push(*c_watcher.read());
    });

    stagger.run();
    wnd.draw_frame();
    AppCtx::run_until_stalled();
    stagger.run();
    AppCtx::run_until_stalled();
    stagger.stop();
    AppCtx::run_until_stalled();

    assert_eq!(&*signals.read(), &[true, false, true, false]);
  }
}
