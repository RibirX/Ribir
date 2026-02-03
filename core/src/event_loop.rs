use std::collections::VecDeque;

use ribir_algo::Rc;
use ribir_geom::Size;
use rxrust::prelude::Observer;
use tokio::{select, sync::mpsc::UnboundedReceiver};
use winit::event::ElementState;

use crate::{
  context::AppCtx,
  ticker::{FrameMsg, Instant},
  window::{UiEvent, Window, WindowId},
};
pub(crate) enum RibirEvent {
  Ui(UiEvent),
  Framework(FrameworkEvent),
}

pub(crate) enum FrameworkEvent {
  DataChanged,

  NewFrame { wnd_id: WindowId, force_redraw: bool },

  BeforeLayout { wnd_id: WindowId },

  Layout { wnd_id: WindowId, wnd_size: Size },

  FrameFinish { wnd_id: WindowId, wnd_size: Size },

  CloseWindow { wnd_id: WindowId },
}

impl From<FrameworkEvent> for RibirEvent {
  fn from(value: FrameworkEvent) -> Self { RibirEvent::Framework(value) }
}

impl From<UiEvent> for RibirEvent {
  fn from(value: UiEvent) -> Self { RibirEvent::Ui(value) }
}

enum EventLoopHandle {
  Idle(IdleHandle),
  FrameLayout(FrameHandle),
}

impl From<IdleHandle> for EventLoopHandle {
  fn from(value: IdleHandle) -> Self { EventLoopHandle::Idle(value) }
}

impl From<FrameHandle> for EventLoopHandle {
  fn from(value: FrameHandle) -> Self { EventLoopHandle::FrameLayout(value) }
}

impl Default for EventLoopHandle {
  fn default() -> Self { IdleHandle::default().into() }
}

pub struct EventLoop {
  handle: EventLoopHandle,
  framework_events: UnboundedReceiver<FrameworkEvent>,
}

impl EventLoop {
  pub(crate) fn new(framework_events: UnboundedReceiver<FrameworkEvent>) -> Self {
    EventLoop { handle: EventLoopHandle::default(), framework_events }
  }

  pub async fn run(self, mut ui_events: UnboundedReceiver<UiEvent>) {
    let mut queue: VecDeque<RibirEvent> = VecDeque::new();
    let Self { mut handle, mut framework_events } = self;
    loop {
      select! {
       event = framework_events.recv() => {
          if let Some(event) = event {
            queue.push_back(event.into());
          } else {
            if let Some(shell) = AppCtx::shell_mut().take(){
              shell.exit();
            }
            break;
          }
        }
        Some(event) = ui_events.recv() => {
          queue.push_back(event.into());
        }
      }
      handle = handle.run(&mut queue);
    }
  }
}

#[derive(Default)]
struct IdleHandle {}

struct FrameHandle {
  wnd: Rc<Window>,
  need_redraw: bool,
  has_data_changed: bool,
  events: Vec<RibirEvent>,
}

impl FrameHandle {
  fn new(wnd: Rc<Window>, force_draw: bool) -> Self {
    let wnd_id = wnd.id();
    let this = Self { wnd, events: vec![], need_redraw: force_draw, has_data_changed: false };

    let mut ticker = this.wnd.frame_ticker.clone();
    ticker.next(FrameMsg::NewFrame(Instant::now()));
    this.wnd.run_frame_tasks();

    AppCtx::send_event(FrameworkEvent::BeforeLayout { wnd_id });
    this
  }

  fn on_event(self, event: RibirEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    match event {
      RibirEvent::Ui(e) => self.on_ui_event(e),
      RibirEvent::Framework(e) => self.on_framework_event(e),
    }
  }

  fn on_framework_event(mut self, event: FrameworkEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    match event {
      FrameworkEvent::NewFrame { wnd_id, force_redraw } => {
        if wnd_id == self.wnd.id() {
          self.need_redraw |= force_redraw;
        } else {
          self.events.push(event.into());
        }
      }
      FrameworkEvent::BeforeLayout { wnd_id } => {
        assert!(wnd_id == self.wnd.id());
        let mut ticker = self.wnd.frame_ticker.clone();
        ticker.next(FrameMsg::BeforeLayout(Instant::now()));
        let wnd_size = self.wnd.size();
        self.wnd.update_painter_viewport();
        AppCtx::send_event(FrameworkEvent::Layout { wnd_id, wnd_size });
      }
      FrameworkEvent::FrameFinish { wnd_id, wnd_size: size } => {
        assert!(wnd_id == self.wnd.id());
        self.layout_ready(size);

        return self.frame_end();
      }
      FrameworkEvent::Layout { wnd_id, wnd_size } => {
        assert!(wnd_id == self.wnd.id());
        let next_event = if self.wnd.tree().is_dirty() {
          self.layout(wnd_size);
          FrameworkEvent::Layout { wnd_id, wnd_size }
        } else {
          FrameworkEvent::FrameFinish { wnd_id, wnd_size }
        };
        AppCtx::send_event(next_event);
      }
      FrameworkEvent::DataChanged => {
        self.has_data_changed = AppCtx::emit_change();
      }
      FrameworkEvent::CloseWindow { .. } => {
        self.events.push(event.into());
      }
    }

    (vec![], self.into())
  }

  fn on_ui_event(mut self, event: UiEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    match event {
      UiEvent::RedrawRequest { wnd_id, force } => {
        if wnd_id != self.wnd.id() {
          self.events.push(event.into());
        } else {
          self.need_redraw |= force;
        }
      }
      _ => self.events.push(event.into()),
    }
    (vec![], EventLoopHandle::FrameLayout(self))
  }

  fn layout(&mut self, size: Size) {
    self.wnd.update_painter_viewport();
    self.wnd.run_frame_tasks();
    if self.wnd.tree().is_dirty() {
      self.wnd.layout(size);
      self.need_redraw = true;
    }
  }

  fn layout_ready(&self, wnd_size: Size) {
    if self.need_redraw {
      self.wnd.draw_frame(Some(wnd_size));
    }
    let mut ticker = self.wnd.frame_ticker.clone();
    ticker.next(FrameMsg::Finish(Instant::now()));
  }

  fn frame_end(self) -> (Vec<RibirEvent>, EventLoopHandle) {
    if self.has_data_changed {
      for wnd in AppCtx::windows().borrow().values() {
        wnd.shell_wnd().borrow().request_draw(false);
      }
    }
    AppCtx::end_frame();
    (self.events, EventLoopHandle::Idle(IdleHandle::default()))
  }
}

impl IdleHandle {
  fn on_event(self, event: RibirEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    match event {
      RibirEvent::Ui(e) => self.on_ui_event(e),
      RibirEvent::Framework(e) => (vec![], self.on_framework_event(e)),
    }
  }
  fn on_ui_event(self, event: UiEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    let wnd_id = event.wnd_id();
    match event {
      UiEvent::RedrawRequest { wnd_id, force } => {
        AppCtx::send_event(FrameworkEvent::NewFrame { wnd_id, force_redraw: force });
      }
      UiEvent::ModifiersChanged { wnd_id, state } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd
            .dispatcher
            .borrow_mut()
            .modifiers_changed(state);
        }
      }
      UiEvent::ReceiveChars { wnd_id, chars } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_receive_chars(chars);
        }
      }
      UiEvent::Resize { .. } => (),
      UiEvent::CursorLeft { wnd_id } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_cursor_leave();
        }
      }
      UiEvent::MouseWheel { wnd_id, delta_x, delta_y } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_wheel(delta_x, delta_y);
        }
      }
      UiEvent::CursorMoved { wnd_id, pos } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_cursor_move(pos);
        }
      }
      UiEvent::ImePreEdit { wnd_id, ime } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_ime(ime);
        }
      }
      UiEvent::KeyBoard { wnd_id, key, state, physical_key, is_repeat, location } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.process_keyboard_event(physical_key, key, is_repeat, location, state);
        }
      }
      UiEvent::MouseInput { wnd_id, device_id, button, state } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          if state == ElementState::Pressed {
            wnd.force_exit_pre_edit()
          }

          match state {
            ElementState::Pressed => wnd.process_mouse_press(device_id, button),
            ElementState::Released => wnd.process_mouse_release(device_id, button),
          }
        }
      }
      UiEvent::CloseRequest { wnd_id } => {
        AppCtx::send_event(FrameworkEvent::CloseWindow { wnd_id });
      }
    }
    if let Some(wnd) = wnd_id.and_then(|id| AppCtx::get_window(id)) {
      wnd.run_frame_tasks();
    }
    (vec![], self.into())
  }

  fn on_framework_event(self, event: FrameworkEvent) -> EventLoopHandle {
    match event {
      FrameworkEvent::DataChanged => {
        let changed = AppCtx::emit_change();
        if changed {
          for wnd in AppCtx::windows().borrow().values() {
            wnd.shell_wnd().borrow().request_draw(false);
          }
        }
      }
      FrameworkEvent::NewFrame { wnd_id, force_redraw } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          return FrameHandle::new(wnd, force_redraw).into();
        }
      }
      FrameworkEvent::CloseWindow { wnd_id } => {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.dispose();
        }
        if !AppCtx::has_wnd() {
          AppCtx::exit();
        }
      }
      FrameworkEvent::BeforeLayout { .. }
      | FrameworkEvent::Layout { .. }
      | FrameworkEvent::FrameFinish { .. } => {
        panic!("Unexpected layout event");
      }
    }
    self.into()
  }
}

impl EventLoopHandle {
  fn on_event(self, event: RibirEvent) -> (Vec<RibirEvent>, EventLoopHandle) {
    match self {
      EventLoopHandle::Idle(wnd_loop) => wnd_loop.on_event(event),
      EventLoopHandle::FrameLayout(layout_loop) => layout_loop.on_event(event),
    }
  }

  pub(crate) fn run(self, events: &mut VecDeque<RibirEvent>) -> EventLoopHandle {
    let mut handle = self;
    while let Some(event) = events.pop_front() {
      let (left_events, new_handle) = handle.on_event(event);
      handle = new_handle;

      for event in left_events.into_iter().rev() {
        events.push_front(event);
      }
    }
    handle
  }
}
