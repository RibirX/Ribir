use crossterm::event::KeyEvent as CrosstermKeyboardInput;
use ribir_core::prelude::KeyboardInput as RibirKeyboardInput;

use crate::{from_element_state::WrappedElementState, prelude::WrappedVirtualKeyCode};

#[derive(Clone)]
pub struct WrappedKeyboardInput(CrosstermKeyboardInput);

impl From<CrosstermKeyboardInput> for WrappedKeyboardInput {
  fn from(value: CrosstermKeyboardInput) -> Self { WrappedKeyboardInput(value) }
}

impl From<WrappedKeyboardInput> for CrosstermKeyboardInput {
  fn from(val: WrappedKeyboardInput) -> Self { val.0 }
}

impl From<WrappedKeyboardInput> for RibirKeyboardInput {
  fn from(val: WrappedKeyboardInput) -> Self {
    RibirKeyboardInput {
      scancode: 0,
      state: WrappedElementState::from(val.0.kind).into(),
      virtual_keycode: WrappedVirtualKeyCode::from(val.0.code).try_into().ok(),
    }
  }
}

// impl From<RibirKeyboardInput> for WrappedKeyboardInput {
//   #[allow(deprecated)]
//   fn from(value: RibirKeyboardInput) -> WrappedKeyboardInput {
//     WrappedKeyboardInput::from(CrosstermKeyboardInput {
//       scancode: value.scancode,
//       state: WrappedElementState::from(value.state).into(),
//       virtual_keycode: value
//         .virtual_keycode
//         .map(|v| WrappedVirtualKeyCode::from(v).into()),
//       modifiers: CrosstermModifiersState::default(),
//     })
//   }
// }

#[cfg(test)]
mod tests {
  // use super::*;

  #[test]
  fn from_crossterm() {

    // let w = WrappedKeyboardInput::from(CrosstermKeyboardInput {
    //   scancode: 64,
    //   state: CrosstermElementState::Pressed,
    //   virtual_keycode: Some(CrosstermVirtualKeyCode::A),
    //   modifiers: CrosstermModifiersState::default(),
    // });
    // let ribir: RibirKeyboardInput = w.clone().into();
    // let winit: RibirKeyboardInput = w.into();
    // assert_eq!(ribir.scancode, 64);
    // assert_eq!(winit.scancode, 64);
  }
}
