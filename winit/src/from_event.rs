
use crate::{
  from_device_id::WrappedPointerId,
  from_element_state::WrappedElementState,
  from_keyboard_input::WrappedKeyboardInput,
  from_modifiers::WrappedModifiersState,
  from_mouse::WrappedMouseButton,
  from_size::{WrappedPhysicalPosition, WrappedPhysicalSize},
  from_touch_phase::WrappedTouchPhase,
  prelude::WrappedMouseScrollDelta,
};
use ribir_core::prelude::WindowEvent as RibirWindowEvent;
use winit::event::{ModifiersState as WinitModifiersState, WindowEvent as WinitWindowEvent};

pub struct WrappedWindowEvent<'a>(WinitWindowEvent<'a>);

impl<'a> From<WinitWindowEvent<'a>> for WrappedWindowEvent<'a> {
  fn from(value: WinitWindowEvent<'a>) -> Self { WrappedWindowEvent(value) }
}

impl<'a> From<WrappedWindowEvent<'a>> for WinitWindowEvent<'a> {
  fn from(val: WrappedWindowEvent<'a>) -> Self { val.0 }
}

impl<'a> From<WrappedWindowEvent<'a>> for RibirWindowEvent {
  fn from(val: WrappedWindowEvent<'a>) -> Self {
    match val.0 {
      WinitWindowEvent::Resized(size) => {
        RibirWindowEvent::Resized(WrappedPhysicalSize::from(size).into())
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
          position: WrappedPhysicalPosition::<f64>::from(position).into(),
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
        delta: WrappedMouseScrollDelta::from(delta).into(),
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

impl<'a> From<RibirWindowEvent> for WrappedWindowEvent<'a> {
  fn from(value: RibirWindowEvent) -> WrappedWindowEvent<'a> {
    let w_event = match value {
      RibirWindowEvent::Resized(size) => {
        WinitWindowEvent::Resized(WrappedPhysicalSize::from(size).into())
      }
      RibirWindowEvent::ReceivedCharacter(char) => WinitWindowEvent::ReceivedCharacter(char),
      RibirWindowEvent::KeyboardInput { device_id, input, is_synthetic } => {
        WinitWindowEvent::KeyboardInput {
          device_id: WrappedPointerId::from(device_id).into(),
          input: WrappedKeyboardInput::from(input).into(),
          is_synthetic,
        }
      }
      RibirWindowEvent::ModifiersChanged(modifiers) => {
        WinitWindowEvent::ModifiersChanged(WrappedModifiersState::from(modifiers).into())
      }

      RibirWindowEvent::CursorMoved { device_id, position } => WinitWindowEvent::CursorMoved {
        device_id: WrappedPointerId::from(device_id).into(),
        position: WrappedPhysicalPosition::<f64>::from(position).into(),
        modifiers: WinitModifiersState::default(),
      },
      RibirWindowEvent::CursorLeft { device_id } => WinitWindowEvent::CursorLeft {
        device_id: WrappedPointerId::from(device_id).into(),
      },

      RibirWindowEvent::MouseWheel { device_id, delta, phase } => WinitWindowEvent::MouseWheel {
        device_id: WrappedPointerId::from(device_id).into(),
        delta: WrappedMouseScrollDelta::from(delta).into(),
        phase: WrappedTouchPhase::from(phase).into(),
        modifiers: WinitModifiersState::default(),
      },

      RibirWindowEvent::MouseInput { device_id, state, button } => WinitWindowEvent::MouseInput {
        device_id: WrappedPointerId::from(device_id).into(),
        state: WrappedElementState::from(state).into(),
        button: WrappedMouseButton::from(button).into(),
        modifiers: WinitModifiersState::default(),
      },
      RibirWindowEvent::ScaleFactorChanged { scale_factor:_, new_inner_size:_ } => {
        // WinitWindowEvent::ScaleFactorChanged {
        //   scale_factor,
        //   new_inner_size: WrappedPhysicalSize::<u32>::from(new_inner_size).into(),
        // }
        panic!("Unimplemented: \"{value:?}\" can not be converted to winit enum.");
      }
      other => {
        panic!("Unimplemented: \"{other:?}\"");
      }
    };
    w_event.into()
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::from_size::WinitPhysicalSize;

  #[test]
  fn from_winit() { WinitWindowEvent::Resized(WinitPhysicalSize::<u32> { width: 5, height: 3 }); }
}
