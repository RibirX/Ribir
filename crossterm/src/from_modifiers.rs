use ribir_core::events::ModifiersState as RibirModifiersState;
// use winit::event::ModifiersState as CrosstermModifiersState;

use crossterm::event::KeyModifiers as CrosstermKeyModifiers;

pub struct WrappedModifiersState(CrosstermKeyModifiers);

impl From<CrosstermKeyModifiers> for WrappedModifiersState {
  fn from(value: CrosstermKeyModifiers) -> Self { WrappedModifiersState(value) }
}

impl From<WrappedModifiersState> for RibirModifiersState {
  fn from(val: WrappedModifiersState) -> Self {
    let shift = if val.0.contains(CrosstermKeyModifiers::SHIFT) {
      RibirModifiersState::SHIFT
    } else {
      RibirModifiersState::empty()
    };

    let ctrl = if val.0.contains(CrosstermKeyModifiers::CONTROL) {
      RibirModifiersState::CTRL
    } else {
      RibirModifiersState::empty()
    };

    let alt = if val
      .0
      .contains(CrosstermKeyModifiers::META | CrosstermKeyModifiers::ALT)
    {
      RibirModifiersState::ALT
    } else {
      RibirModifiersState::empty()
    };

    let logo = if val.0.contains(CrosstermKeyModifiers::SUPER) {
      RibirModifiersState::LOGO
    } else {
      RibirModifiersState::empty()
    };

    shift | ctrl | alt | logo
  }
}
