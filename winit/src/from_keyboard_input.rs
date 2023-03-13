use ribir_core::prelude::KeyboardInput as RibirKeyboardInput;
use winit::{
  event::KeyboardInput as WinitKeyboardInput, event::ModifiersState as WinitModifiersState,
};

use crate::prelude::{WrappedElementState, WrappedVirtualKeyCode};

pub struct WrappedKeyboardInput(WinitKeyboardInput);

impl From<WinitKeyboardInput> for WrappedKeyboardInput {
  fn from(value: WinitKeyboardInput) -> Self { WrappedKeyboardInput(value) }
}

impl From<WrappedKeyboardInput> for WinitKeyboardInput {
  fn from(val: WrappedKeyboardInput) -> Self { val.0 }
}

impl From<WrappedKeyboardInput> for RibirKeyboardInput {
  fn from(val: WrappedKeyboardInput) -> Self {
    RibirKeyboardInput {
      scancode: val.0.scancode,
      state: WrappedElementState::from(val.0.state).into(),
      virtual_keycode: val
        .0
        .virtual_keycode
        .map(|v| WrappedVirtualKeyCode::from(v).into()),
    }
  }
}

impl From<RibirKeyboardInput> for WrappedKeyboardInput {
  #[allow(deprecated)]
  fn from(value: RibirKeyboardInput) -> WrappedKeyboardInput {
    WrappedKeyboardInput::from(WinitKeyboardInput {
      scancode: value.scancode,
      state: WrappedElementState::from(value.state).into(),
      virtual_keycode: value
        .virtual_keycode
        .map(|v| WrappedVirtualKeyCode::from(v).into()),
      modifiers: WinitModifiersState::default(),
    })
    .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use winit::{
    event::ElementState as WinitElementState, event::KeyboardInput as WinitKeyboardInput,
    event::VirtualKeyCode as WinitVirtualKeyCode,
  };

  #[test]
  fn from_winit() {
    #[allow(deprecated)]
    let w = WrappedKeyboardInput::from(WinitKeyboardInput {
      scancode: 64,
      state: WinitElementState::Pressed,
      virtual_keycode: Some(WinitVirtualKeyCode::A),

      modifiers: WinitModifiersState::default(),
    });
    let ribir: RibirKeyboardInput = w.into();
    let winit: RibirKeyboardInput = w.into();
    assert_eq!(ribir.scancode, 64);
    assert_eq!(winit.scancode, 64);
  }
}
