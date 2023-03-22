use crate::{
  from_device_id::WrappedPointerId,
  from_element_state::WrappedElementState,
  from_keyboard_input::WrappedKeyboardInput,
  from_modifiers::WrappedModifiersState,
  from_mouse::WrappedMouseButton,
  from_touch_phase::WrappedTouchPhase,
  prelude::{WrappedLogicalPosition, WrappedLogicalSize, WrappedMouseScrollDelta},
};
use ribir_core::prelude::WindowEvent as RibirWindowEvent;
use winit::event::WindowEvent as WinitWindowEvent;

pub type ScaleToLogicalFactor = f64;

pub struct WrappedWindowEvent<'a>(WinitWindowEvent<'a>, ScaleToLogicalFactor);

impl<'a> From<(WinitWindowEvent<'a>, ScaleToLogicalFactor)> for WrappedWindowEvent<'a> {
  fn from(value: (WinitWindowEvent<'a>, ScaleToLogicalFactor)) -> Self {
    WrappedWindowEvent(value.0, value.1)
  }
}

impl<'a> From<WrappedWindowEvent<'a>> for RibirWindowEvent {
  fn from(val: WrappedWindowEvent<'a>) -> Self {
    match val.0 {
      WinitWindowEvent::Resized(size) => {
        RibirWindowEvent::Resized(WrappedLogicalSize::<f64>::from(size.to_logical(val.1)).into())
      }
      WinitWindowEvent::ReceivedCharacter(char) => RibirWindowEvent::ReceivedCharacter(char),
      WinitWindowEvent::KeyboardInput { device_id, input, is_synthetic } => {
        RibirWindowEvent::KeyboardInput {
          device_id: Box::new(WrappedPointerId::from(device_id)),
          input: WrappedKeyboardInput::from(input).into(),
          is_synthetic,
        }
      }
      WinitWindowEvent::ModifiersChanged(modifiers) => {
        RibirWindowEvent::ModifiersChanged(WrappedModifiersState::from(modifiers).into())
      }
      #[allow(deprecated)]
      WinitWindowEvent::CursorMoved { device_id, position, modifiers: _ } => {
        RibirWindowEvent::CursorMoved {
          device_id: Box::new(WrappedPointerId::from(device_id)),
          position: WrappedLogicalPosition::<f64>::from(position.to_logical(val.1)).into(),
        }
      }
      WinitWindowEvent::CursorLeft { device_id } => RibirWindowEvent::CursorLeft {
        device_id: Box::new(WrappedPointerId::from(device_id)),
      },
      #[allow(deprecated)]
      WinitWindowEvent::MouseWheel {
        device_id,
        delta,
        phase,
        modifiers: _,
      } => RibirWindowEvent::MouseWheel {
        device_id: Box::new(WrappedPointerId::from(device_id)),
        delta: WrappedMouseScrollDelta::from((delta, val.1)).into(),
        phase: WrappedTouchPhase::from(phase).into(),
      },
      #[allow(deprecated)]
      WinitWindowEvent::MouseInput {
        device_id,
        state,
        button,
        modifiers: _,
      } => RibirWindowEvent::MouseInput {
        device_id: Box::new(WrappedPointerId::from(device_id)),
        state: WrappedElementState::from(state).into(),
        button: WrappedMouseButton::from(button).into(),
      },
      // TODO(zoech)
      // WinitWindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
      //   RibirWindowEvent::ScaleFactorChanged {
      //     scale_factor,
      //     new_inner_size: WrappedPhysicalSize::<u32>::from(new_inner_size).into(),
      //   }
      // }
      _ => RibirWindowEvent::Unsupported,
    }
  }
}

#[cfg(test)]
mod tests {

  // use super::*;
  // use crate::from_size::WinitPhysicalSize;

  // #[test]
  // fn from_winit() { WinitWindowEvent::Resized(WinitPhysicalSize::<u32> {
  // width: 5, height: 3 }); }
}
