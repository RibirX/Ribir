use std::collections::HashMap;

use ribir_algo::Rc;
use rxrust::prelude::Observer;
use tokio::{
  sync::mpsc::{UnboundedReceiver, error::TryRecvError},
  task::yield_now,
};
use winit::event::ElementState;

use crate::{
  context::AppCtx,
  ticker::{FrameMsg, Instant},
  window::{UiEvent, Window, WindowId},
};

#[cfg(debug_assertions)]
const MAX_LAYOUT_ITERS: usize = usize::MAX;
#[cfg(not(debug_assertions))]
const MAX_LAYOUT_ITERS: usize = 128;

pub enum CoreMsg {
  Platform(UiEvent),
  FrameReady {
    wnd_id: WindowId,
    force: bool,
  },
  /// Wake the scheduler so it can globally flush pending reactive changes.
  DataChanged,
  CloseWindow {
    wnd_id: WindowId,
  },
  Exit,
}

impl From<UiEvent> for CoreMsg {
  fn from(event: UiEvent) -> Self {
    match event {
      UiEvent::RedrawRequest { wnd_id, force } => Self::FrameReady { wnd_id, force },
      event => Self::Platform(event),
    }
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum WindowPhase {
  #[default]
  Idle,
  RedrawPending,
  InFrame,
}

#[derive(Clone, Debug, Default)]
struct WindowController {
  phase: WindowPhase,
  frame_ready: bool,
  force_redraw: bool,
}

#[derive(Default)]
struct Scheduler {
  controllers: HashMap<WindowId, WindowController>,
  active_frame: Option<WindowId>,
}

pub struct EventLoop {
  events: UnboundedReceiver<CoreMsg>,
  scheduler: Scheduler,
}

impl EventLoop {
  pub(crate) fn new(events: UnboundedReceiver<CoreMsg>) -> Self {
    Self { events, scheduler: Scheduler::default() }
  }

  pub async fn run(mut self) {
    loop {
      if self.scheduler.has_ready_frame() {
        self.scheduler.run_ready_frame().await;
        continue;
      }

      let Some(event) = self.events.recv().await else {
        self.shutdown_shell();
        break;
      };

      if !self.on_core_msg(event) {
        break;
      }

      while !self.scheduler.has_ready_frame() {
        match self.events.try_recv() {
          Ok(event) => {
            if !self.on_core_msg(event) {
              return;
            }
          }
          Err(TryRecvError::Empty) => break,
          Err(TryRecvError::Disconnected) => {
            self.shutdown_shell();
            return;
          }
        }
      }
    }
  }

  fn shutdown_shell(&mut self) {
    if let Some(shell) = AppCtx::shell_mut().take() {
      shell.exit();
    }
  }

  fn on_core_msg(&mut self, event: CoreMsg) -> bool {
    match event {
      CoreMsg::Platform(event) => {
        self.on_platform_event(event);
      }
      CoreMsg::FrameReady { wnd_id, force } => {
        if let Some(wnd) = self.scheduler.get_window(wnd_id) {
          self.scheduler.on_redraw_ready(wnd, force);
        }
      }
      CoreMsg::DataChanged => {
        self.scheduler.flush_global_changes();
      }
      CoreMsg::CloseWindow { wnd_id } => {
        if let Some(wnd) = self.scheduler.get_window(wnd_id) {
          self.scheduler.close_window(wnd);
        }
      }
      CoreMsg::Exit => {
        self.shutdown_shell();
        return false;
      }
    }
    true
  }

  fn on_platform_event(&mut self, event: UiEvent) {
    let Some(wnd_id) = event.wnd_id() else { return };
    let Some(wnd) = self.scheduler.get_window(wnd_id) else { return };

    match event {
      UiEvent::RedrawRequest { force, .. } => {
        self.scheduler.on_redraw_ready(wnd, force);
        return;
      }
      UiEvent::Resize { .. } => {
        self.scheduler.on_resize(wnd);
        return;
      }
      UiEvent::CloseRequest { .. } => {
        self.scheduler.close_window(wnd);
        return;
      }
      UiEvent::ModifiersChanged { state, .. } => wnd
        .dispatcher
        .borrow_mut()
        .modifiers_changed(state),
      UiEvent::ReceiveChars { chars, .. } => wnd.process_receive_chars(chars),
      UiEvent::CursorLeft { .. } => wnd.process_cursor_leave(),
      UiEvent::MouseWheel { delta_x, delta_y, .. } => wnd.process_wheel(delta_x, delta_y),
      UiEvent::CursorMoved { pos, .. } => wnd.process_cursor_move(pos),
      UiEvent::ImePreEdit { ime, .. } => wnd.process_ime(ime),
      UiEvent::KeyBoard { physical_key, key, is_repeat, location, state, .. } => {
        wnd.process_keyboard_event(physical_key, key, is_repeat, location, state);
      }
      UiEvent::MouseInput { device_id, button, state, .. } => match state {
        ElementState::Pressed => {
          wnd.force_exit_pre_edit();
          wnd.process_mouse_press(device_id, button);
        }
        ElementState::Released => wnd.process_mouse_release(device_id, button),
      },
    }

    self.scheduler.finish_window_input(wnd);
  }
}

impl Scheduler {
  fn controller_mut(&mut self, wnd_id: WindowId) -> &mut WindowController {
    self.controllers.entry(wnd_id).or_default()
  }

  fn clear_closed_window(&mut self, wnd_id: WindowId) { self.controllers.remove(&wnd_id); }

  /// Return the window if it still exists; if not, clean up any stale
  /// controller state for that id.
  fn get_window(&mut self, wnd_id: WindowId) -> Option<Rc<Window>> {
    AppCtx::get_window(wnd_id).or_else(|| {
      self.clear_closed_window(wnd_id);
      None
    })
  }

  /// Emit a ticker message for `wnd`.
  fn tick(wnd: &Window, msg: FrameMsg) { wnd.frame_ticker.clone().next(msg); }

  fn flush_global_changes(&mut self) -> bool {
    let changed = AppCtx::emit_change();
    if !changed {
      return false;
    }

    let wnd_ids: Vec<_> = AppCtx::windows()
      .borrow()
      .keys()
      .copied()
      .collect();
    for wnd_id in wnd_ids {
      if let Some(wnd) = self.get_window(wnd_id) {
        self.schedule_platform_redraw(wnd, false);
      }
    }

    true
  }

  fn finish_window_input(&mut self, wnd: Rc<Window>) {
    self.drain_frame_queue(&wnd);
    if !self.flush_global_changes() {
      self.schedule_platform_redraw(wnd, false);
    }
  }

  fn on_resize(&mut self, wnd: Rc<Window>) { self.schedule_platform_redraw(wnd, false); }

  fn schedule_platform_redraw(&mut self, wnd: Rc<Window>, force: bool) {
    let wnd_id = wnd.id();
    let controller = self.controller_mut(wnd_id);
    controller.force_redraw |= force;

    if !matches!(controller.phase, WindowPhase::Idle) {
      return;
    }

    wnd.shell_wnd().borrow().request_draw(force);
    controller.frame_ready = false;
    controller.phase = WindowPhase::RedrawPending;
  }

  fn on_redraw_ready(&mut self, wnd: Rc<Window>, force: bool) {
    let wnd_id = wnd.id();
    let controller = self.controller_mut(wnd_id);
    controller.force_redraw |= force;

    if matches!(controller.phase, WindowPhase::InFrame) {
      // Keep a redraw request that arrives during a frame so it can schedule
      // the following frame after we return to `Idle`.
      controller.force_redraw = true;
      return;
    }

    controller.phase = WindowPhase::RedrawPending;
    controller.frame_ready = true;
  }

  fn has_ready_frame(&self) -> bool {
    self.controllers.values().any(|controller| {
      matches!(controller.phase, WindowPhase::RedrawPending) && controller.frame_ready
    })
  }

  async fn run_ready_frame(&mut self) -> bool {
    let ready_wnd_ids: Vec<_> = self
      .controllers
      .iter()
      .filter_map(|(wnd_id, controller)| {
        (matches!(controller.phase, WindowPhase::RedrawPending) && controller.frame_ready)
          .then_some(*wnd_id)
      })
      .collect();

    for wnd_id in ready_wnd_ids {
      let Some(wnd) = self.get_window(wnd_id) else {
        continue;
      };

      {
        let controller = self.controller_mut(wnd_id);
        controller.phase = WindowPhase::InFrame;
        controller.frame_ready = false;
      }

      self.active_frame = Some(wnd_id);
      self.run_window_frame(wnd).await;
      return true;
    }

    false
  }

  async fn run_window_frame(&mut self, wnd: Rc<Window>) {
    let wnd_id = wnd.id();
    let mut need_redraw = std::mem::take(&mut self.controller_mut(wnd_id).force_redraw);

    Scheduler::tick(&wnd, FrameMsg::NewFrame(Instant::now()));
    self.drain_frame_work(&wnd).await;

    Scheduler::tick(&wnd, FrameMsg::BeforeLayout(Instant::now()));
    wnd.update_painter_viewport();
    self.drain_frame_work(&wnd).await;

    let mut notified_widgets = ahash::HashSet::default();
    let mut layout_converged = false;
    for _ in 0..MAX_LAYOUT_ITERS {
      if wnd.tree().is_dirty() {
        need_redraw |= wnd.layout(wnd.size(), &mut notified_widgets);
      }

      self.drain_frame_work(&wnd).await;

      if wnd.tree().is_dirty() || AppCtx::has_pending_changes() {
        continue;
      }

      {
        let tree = wnd.tree_mut();
        wnd
          .focus_mgr
          .borrow_mut()
          .on_widget_tree_update(tree);
      }
      self.drain_frame_work(&wnd).await;

      if wnd.tree().is_dirty() || AppCtx::has_pending_changes() {
        continue;
      }

      Scheduler::tick(&wnd, FrameMsg::LayoutReady(Instant::now()));
      self.drain_frame_work(&wnd).await;

      if wnd.tree().is_dirty() || AppCtx::has_pending_changes() {
        continue;
      }

      layout_converged = true;
      break;
    }

    assert!(layout_converged, "Layout failed to converge within {MAX_LAYOUT_ITERS} iterations");

    if need_redraw || wnd.need_draw() {
      wnd.draw_frame(Some(wnd.size()));
    }

    Scheduler::tick(&wnd, FrameMsg::Finish(Instant::now()));
    self.drain_frame_work(&wnd).await;

    AppCtx::end_frame();
    self.active_frame = None;

    if self.get_window(wnd_id).is_none() {
      return;
    }

    self.controller_mut(wnd_id).phase = WindowPhase::Idle;
    self.schedule_platform_redraw(wnd, false);
  }

  async fn drain_frame_work(&mut self, wnd: &Window) {
    self.drain_frame_queue(wnd);
    yield_now().await;
    self.flush_global_changes();
  }

  fn close_window(&mut self, wnd: Rc<Window>) {
    self.clear_closed_window(wnd.id());
    wnd.dispose();
    if !AppCtx::has_wnd() {
      AppCtx::exit();
    }
  }

  fn drain_frame_queue(&self, wnd: &Window) { wnd.run_frame_tasks(); }
}

#[cfg(test)]
mod tests {
  use super::{Scheduler, WindowPhase};
  use crate::{prelude::*, reset_test_env, test_helper::TestWindow};

  #[test]
  fn pending_redraws_are_deduplicated_and_force_is_upgraded() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();

    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();
    scheduler.schedule_platform_redraw(wnd_ref.clone(), false);
    scheduler.schedule_platform_redraw(wnd_ref, true);

    assert_eq!(wnd.request_draw_count(), 1);

    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::RedrawPending);
    assert!(!controller.frame_ready);
    assert!(controller.force_redraw);
  }

  #[test]
  fn redraw_requested_during_frame_is_rescheduled_after_frame() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();
    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::InFrame;
    scheduler.on_redraw_ready(wnd_ref.clone(), false);
    assert!(
      scheduler
        .controllers
        .get(&wnd.id())
        .unwrap()
        .force_redraw
    );

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::Idle;
    scheduler.schedule_platform_redraw(wnd_ref, false);

    assert_eq!(wnd.request_draw_count(), 1);
    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::RedrawPending);
    assert!(!controller.frame_ready);
  }
}
