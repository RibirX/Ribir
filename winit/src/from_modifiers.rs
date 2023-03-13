use ribir_core::events::ModifiersState as RibirModifiersState;
use winit::event::ModifiersState as WinitModifiersState;

pub struct WrappedModifiersState(WinitModifiersState);

impl From<WinitModifiersState> for WrappedModifiersState {
  fn from(value: WinitModifiersState) -> Self { WrappedModifiersState(value) }
}

impl From<WrappedModifiersState> for WinitModifiersState {
  fn from(val: WrappedModifiersState) -> Self { val.0 }
}

impl From<WrappedModifiersState> for RibirModifiersState {
  fn from(val: WrappedModifiersState) -> Self {
    let shift = if val.0.shift() {
      RibirModifiersState::SHIFT
    } else {
      RibirModifiersState::empty()
    };

    let ctrl = if val.0.ctrl() {
      RibirModifiersState::CTRL
    } else {
      RibirModifiersState::empty()
    };

    let alt = if val.0.alt() {
      RibirModifiersState::ALT
    } else {
      RibirModifiersState::empty()
    };

    let logo = if val.0.logo() {
      RibirModifiersState::LOGO
    } else {
      RibirModifiersState::empty()
    };

    shift | ctrl | alt | logo
  }
}

impl From<RibirModifiersState> for WrappedModifiersState {
  fn from(value: RibirModifiersState) -> WrappedModifiersState {
    let shift: WinitModifiersState = if value.shift() {
      WinitModifiersState::SHIFT
    } else {
      WinitModifiersState::empty()
    };

    let ctrl: WinitModifiersState = if value.ctrl() {
      WinitModifiersState::CTRL
    } else {
      WinitModifiersState::empty()
    };

    let alt: WinitModifiersState = if value.alt() {
      WinitModifiersState::ALT
    } else {
      WinitModifiersState::empty()
    };

    let logo: WinitModifiersState = if value.logo() {
      WinitModifiersState::LOGO
    } else {
      WinitModifiersState::empty()
    };

    (shift | ctrl | alt | logo).into()
  }
}
