use crate::{
  from_device_id::CrosstermPointerId, from_keyboard_input::WrappedKeyboardInput,
  from_mouse::WrappedMouseButton,
};
use crossterm::event::{Event as CrosstermWindowEvent, MouseEventKind};
use ribir_core::prelude::{
  ElementState, MouseScrollDelta, TouchPhase, WindowEvent as RibirWindowEvent,
};

pub struct WrappedWindowEvent(CrosstermWindowEvent);

impl From<CrosstermWindowEvent> for WrappedWindowEvent {
  fn from(value: CrosstermWindowEvent) -> Self { WrappedWindowEvent(value) }
}

impl From<WrappedWindowEvent> for RibirWindowEvent {
  fn from(val: WrappedWindowEvent) -> Self {
    match val.0 {
      CrosstermWindowEvent::FocusGained => RibirWindowEvent::Unsupported,
      CrosstermWindowEvent::FocusLost => RibirWindowEvent::CursorLeft {
        device_id: Box::new(CrosstermPointerId::zero()),
      },
      CrosstermWindowEvent::Key(event) => RibirWindowEvent::KeyboardInput {
        device_id: Box::new(CrosstermPointerId::zero()),
        input: WrappedKeyboardInput::from(event).into(),
        is_synthetic: false,
      },
      CrosstermWindowEvent::Mouse(event) => match event.kind {
        MouseEventKind::Down(button) => RibirWindowEvent::MouseInput {
          device_id: Box::new(CrosstermPointerId::zero()),
          state: ElementState::Pressed,
          button: WrappedMouseButton::from(button).into(),
        },
        MouseEventKind::Up(button) => RibirWindowEvent::MouseInput {
          device_id: Box::new(CrosstermPointerId::zero()),
          state: ElementState::Released,
          button: WrappedMouseButton::from(button).into(),
        },
        MouseEventKind::Drag(_) => RibirWindowEvent::Unsupported,
        MouseEventKind::Moved => RibirWindowEvent::CursorMoved {
          device_id: Box::new(CrosstermPointerId::zero()),
          position: (event.column as f32, event.row as f32).into(),
        },
        MouseEventKind::ScrollDown => RibirWindowEvent::MouseWheel {
          device_id: Box::new(CrosstermPointerId::zero()),
          delta: MouseScrollDelta::LineDelta(0., 1.),
          phase: TouchPhase::Moved,
        },
        MouseEventKind::ScrollUp => RibirWindowEvent::MouseWheel {
          device_id: Box::new(CrosstermPointerId::zero()),
          delta: MouseScrollDelta::LineDelta(0., -1.),
          phase: TouchPhase::Moved,
        },
      },
      #[cfg(feature = "bracketed-paste")]
      CrosstermWindowEvent::Paste(_data) => RibirWindowEvent::Unsupported,
      CrosstermWindowEvent::Resize(width, height) => {
        RibirWindowEvent::Resized((width as f32, height as f32).into())
      }
    }
  }
}
