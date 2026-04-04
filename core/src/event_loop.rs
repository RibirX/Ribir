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
  window::{RedrawDemand, UiEvent, Window, WindowId},
};

#[cfg(debug_assertions)]
const MAX_LAYOUT_ITERS: usize = usize::MAX;
#[cfg(not(debug_assertions))]
const MAX_LAYOUT_ITERS: usize = 128;

pub enum CoreMsg {
  Platform(UiEvent),
  FrameReady {
    wnd_id: WindowId,
    demand: RedrawDemand,
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
      UiEvent::RedrawRequest { wnd_id, demand } => Self::FrameReady { wnd_id, demand },
      event => Self::Platform(event),
    }
  }
}

/// Core redraw scheduling model.
///
/// The event loop tracks each window on two orthogonal axes:
///
/// 1. `WindowPhase` describes the handshake with the platform: `Idle ->
///    PlatformRequested -> Ready -> InFrame`.
/// 2. `RedrawDemand` describes whether the window still owes another frame, and
///    whether that frame must be forced.
///
/// High-level flow:
///
/// - Widget/data changes call `request_redraw`, which promotes the window's
///   `RedrawDemand` and, if the window is `Idle`, ensures the platform has been
///   asked for a redraw.
/// - When the platform delivers `UiEvent::RedrawRequest`, the controller moves
///   to `Ready`.
/// - `run_ready_frame` consumes the current demand, runs layout/draw work, and
///   allows frame hooks to enqueue more demand while the phase is `InFrame`.
/// - After the frame completes, if the frame did draw, demand is promoted back
///   to `Normal` so the scheduler can run one follow-up frame to reach a stable
///   state. If the frame did not draw and no new demand arrived, the window
///   stays `Idle` and redraws stop.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum WindowPhase {
  #[default]
  Idle,
  PlatformRequested,
  Ready,
  InFrame,
}

#[derive(Clone, Debug, Default)]
struct WindowController {
  phase: WindowPhase,
  demand: RedrawDemand,
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
      CoreMsg::FrameReady { wnd_id, demand } => {
        if let Some(wnd) = self.scheduler.get_window(wnd_id) {
          self.scheduler.on_redraw_ready(wnd, demand);
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
    #[cfg(feature = "debug")]
    crate::debug_tool::record_ui_event(&event);

    let Some(wnd_id) = event.wnd_id() else { return };
    let Some(wnd) = self.scheduler.get_window(wnd_id) else { return };

    match event {
      UiEvent::RedrawRequest { demand, .. } => {
        self.scheduler.on_redraw_ready(wnd, demand);
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
        self.request_redraw(wnd, RedrawDemand::Normal);
      }
    }

    true
  }

  fn finish_window_input(&mut self, wnd: Rc<Window>) {
    self.drain_frame_queue(&wnd);
    if !self.flush_global_changes() {
      self.request_redraw(wnd, RedrawDemand::Normal);
    }
  }

  fn on_resize(&mut self, wnd: Rc<Window>) { self.request_redraw(wnd, RedrawDemand::Normal); }

  fn request_redraw(&mut self, wnd: Rc<Window>, demand: RedrawDemand) {
    let wnd_id = wnd.id();
    let controller = self.controller_mut(wnd_id);
    controller.demand.promote(demand);

    if !matches!(controller.phase, WindowPhase::Idle) {
      return;
    }

    self.ensure_platform_redraw_requested(wnd);
  }

  fn ensure_platform_redraw_requested(&mut self, wnd: Rc<Window>) {
    let wnd_id = wnd.id();
    let controller = self.controller_mut(wnd_id);
    if !matches!(controller.phase, WindowPhase::Idle) || !controller.demand.requires_frame() {
      return;
    }

    wnd
      .shell_wnd()
      .borrow()
      .request_draw(controller.demand);
    controller.phase = WindowPhase::PlatformRequested;
  }

  fn on_redraw_ready(&mut self, wnd: Rc<Window>, demand: RedrawDemand) {
    let wnd_id = wnd.id();
    let controller = self.controller_mut(wnd_id);
    controller.demand.promote(demand);

    if matches!(controller.phase, WindowPhase::InFrame) {
      return;
    }

    controller.phase = WindowPhase::Ready;
  }

  fn has_ready_frame(&self) -> bool {
    self
      .controllers
      .values()
      .any(|controller| matches!(controller.phase, WindowPhase::Ready))
  }

  async fn run_ready_frame(&mut self) -> bool {
    let ready_wnd_ids: Vec<_> = self
      .controllers
      .iter()
      .filter_map(|(wnd_id, controller)| {
        matches!(controller.phase, WindowPhase::Ready).then_some(*wnd_id)
      })
      .collect();

    for wnd_id in ready_wnd_ids {
      let Some(wnd) = self.get_window(wnd_id) else {
        continue;
      };

      {
        let controller = self.controller_mut(wnd_id);
        controller.phase = WindowPhase::InFrame;
      }

      self.active_frame = Some(wnd_id);
      self.run_window_frame(wnd).await;
      return true;
    }

    false
  }

  async fn run_window_frame(&mut self, wnd: Rc<Window>) {
    let wnd_id = wnd.id();
    let frame_demand = std::mem::take(&mut self.controller_mut(wnd_id).demand);
    let mut need_redraw = frame_demand.requires_forced_draw();

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

    let did_draw_frame = (need_redraw || wnd.need_draw()) && wnd.draw_frame(Some(wnd.size()));

    Scheduler::tick(&wnd, FrameMsg::Finish(Instant::now()));
    self.drain_frame_work(&wnd).await;

    AppCtx::end_frame();
    self.active_frame = None;

    if self.get_window(wnd_id).is_none() {
      return;
    }

    {
      let controller = self.controller_mut(wnd_id);
      if did_draw_frame {
        controller.demand.promote(RedrawDemand::Normal);
      }
      controller.phase = WindowPhase::Idle;
    }

    self.ensure_platform_redraw_requested(wnd);
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
  use super::{RedrawDemand, Scheduler, WindowPhase};
  use crate::{prelude::*, reset_test_env, test_helper::TestWindow};

  #[test]
  fn pending_redraws_are_deduplicated_and_force_is_upgraded() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();

    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();
    scheduler.request_redraw(wnd_ref.clone(), RedrawDemand::Normal);
    scheduler.request_redraw(wnd_ref, RedrawDemand::Force);

    assert_eq!(wnd.request_draw_count(), 1);

    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::PlatformRequested);
    assert_eq!(controller.demand, RedrawDemand::Force);
  }

  #[test]
  fn redraw_requested_during_frame_is_rescheduled_after_frame() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();
    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::InFrame;
    scheduler.on_redraw_ready(wnd_ref.clone(), RedrawDemand::Normal);
    assert_eq!(
      scheduler
        .controllers
        .get(&wnd.id())
        .unwrap()
        .demand,
      RedrawDemand::Normal
    );

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::Idle;
    scheduler.ensure_platform_redraw_requested(wnd_ref);

    assert_eq!(wnd.request_draw_count(), 1);
    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::PlatformRequested);
    assert_eq!(controller.demand, RedrawDemand::Normal);
  }

  #[test]
  fn drawn_frame_only_requests_one_followup_redraw() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();
    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();

    scheduler.on_redraw_ready(wnd_ref.clone(), RedrawDemand::Normal);
    assert!(AppCtx::run_until(scheduler.run_ready_frame()));
    assert_eq!(wnd.request_draw_count(), 1);

    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::PlatformRequested);
    assert_eq!(controller.demand, RedrawDemand::Normal);

    scheduler.on_redraw_ready(wnd_ref, RedrawDemand::Normal);
    assert!(AppCtx::run_until(scheduler.run_ready_frame()));
    assert_eq!(wnd.request_draw_count(), 1);

    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::Idle);
    assert_eq!(controller.demand, RedrawDemand::None);
  }

  #[test]
  fn non_force_redraw_requested_during_frame_is_rescheduled_after_frame() {
    reset_test_env!();
    let wnd = TestWindow::from_widget(fn_widget! { @Void {} });
    let mut scheduler = Scheduler::default();
    let wnd_ref = AppCtx::get_window(wnd.id()).unwrap();

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::InFrame;
    scheduler.request_redraw(wnd_ref.clone(), RedrawDemand::Normal);
    assert_eq!(wnd.request_draw_count(), 0);
    assert_eq!(
      scheduler
        .controllers
        .get(&wnd.id())
        .unwrap()
        .demand,
      RedrawDemand::Normal
    );

    scheduler.controller_mut(wnd.id()).phase = WindowPhase::Idle;
    scheduler.ensure_platform_redraw_requested(wnd_ref);
    assert_eq!(wnd.request_draw_count(), 1);

    let controller = scheduler.controllers.get(&wnd.id()).unwrap();
    assert_eq!(controller.phase, WindowPhase::PlatformRequested);
    assert_eq!(controller.demand, RedrawDemand::Normal);
  }
}
