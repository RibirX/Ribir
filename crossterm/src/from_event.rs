use crate::{
  from_device_id::CrosstermPointerId, from_keyboard_input::WrappedKeyboardInput,
  from_mouse::WrappedMouseButton,
};
use crossterm::event::{Event as CrosstermWindowEvent, MouseEventKind};
use ribir_core::prelude::{
  ElementState, MouseScrollDelta, TouchPhase, WindowEvent as RibirWindowEvent,
};
use ribir_geometry::{DevicePoint, DeviceSize};

pub struct WrappedWindowEvent(CrosstermWindowEvent);

impl From<CrosstermWindowEvent> for WrappedWindowEvent {
  fn from(value: CrosstermWindowEvent) -> Self { WrappedWindowEvent(value) }
}

impl From<WrappedWindowEvent> for CrosstermWindowEvent {
  fn from(val: WrappedWindowEvent) -> Self { val.0 }
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
          position: DevicePoint::new(event.column as u32, event.row as u32),
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
        RibirWindowEvent::Resized(DeviceSize::new(width as u32, height as u32))
      } /* CrosstermWindowEvent::ReceivedCharacter(char) =>
         * RibirWindowEvent::ReceivedCharacter(char), CrosstermWindowEvent::KeyboardInput
         * { device_id, input, is_synthetic } => {   RibirWindowEvent::KeyboardInput {
         *     device_id: Box::new(CrosstermPointerId::from(device_id)),
         *     input: WrappedKeyboardInput::from(input).into(),
         *     is_synthetic,
         *   }
         * }
         * CrosstermWindowEvent::ModifiersChanged(modifiers) => {
         *   RibirWindowEvent::ModifiersChanged(WrappedModifiersState::from(modifiers).into())
         * }
         * #[allow(deprecated)]
         * CrosstermWindowEvent::CursorMoved { device_id, position, modifiers: _ } => {
         *   RibirWindowEvent::CursorMoved {
         *     device_id: Box::new(CrosstermPointerId::from(device_id)),
         *     position,
         *   }
         * }
         * CrosstermWindowEvent::CursorLeft { device_id } => RibirWindowEvent::CursorLeft {
         *   device_id: Box::new(CrosstermPointerId::from(device_id)),
         * },
         * #[allow(deprecated)]
         * CrosstermWindowEvent::MouseWheel {
         *   device_id,
         *   delta,
         *   phase,
         *   modifiers: _,
         * } => RibirWindowEvent::MouseWheel {
         *   device_id: Box::new(CrosstermPointerId::from(device_id)),
         *   delta: WrappedMouseScrollDelta::from(delta).into(),
         *   phase: WrappedTouchPhase::from(phase).into(),
         * },
         * #[allow(deprecated)]
         * CrosstermWindowEvent::MouseInput {
         *   device_id,
         *   state,
         *   button,
         *   modifiers: _,
         * } => RibirWindowEvent::MouseInput {
         *   device_id: Box::new(CrosstermPointerId::from(device_id)),
         *   state: WrappedElementState::from(state).into(),
         *   button: WrappedMouseButton::from(button).into(),
         * },
         * CrosstermWindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
         *   RibirWindowEvent::ScaleFactorChanged {
         *     scale_factor,
         *     new_inner_size: WrappedPhysicalSize::<u32>::from(new_inner_size).into(),
         *   }
         * } */
    }
  }
}
