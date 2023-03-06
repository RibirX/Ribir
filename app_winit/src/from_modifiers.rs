use ribir_core::events::ModifiersState as CModifiersState;
use winit::event::ModifiersState as WModifiersState;

pub struct RModifiersState(WModifiersState);

impl From<WModifiersState> for RModifiersState {
  fn from(value: WModifiersState) -> Self { RModifiersState(value) }
}

impl From<RModifiersState> for WModifiersState {
  fn from(val: RModifiersState) -> Self { val.0 }
}

impl From<RModifiersState> for CModifiersState {
  fn from(val: RModifiersState) -> Self {
    let shift = if val.0.shift() {
      CModifiersState::SHIFT
    } else {
      CModifiersState::empty()
    };

    let ctrl = if val.0.ctrl() {
      CModifiersState::CTRL
    } else {
      CModifiersState::empty()
    };

    let alt = if val.0.alt() {
      CModifiersState::ALT
    } else {
      CModifiersState::empty()
    };

    let logo = if val.0.logo() {
      CModifiersState::LOGO
    } else {
      CModifiersState::empty()
    };

    shift | ctrl | alt | logo
  }
}

impl From<CModifiersState> for RModifiersState {
  fn from(value: CModifiersState) -> RModifiersState {
    let shift:WModifiersState = if value.shift() {
      WModifiersState::SHIFT
    } else {
      WModifiersState::empty()
    };

    let ctrl :WModifiersState= if value.ctrl() {
      WModifiersState::CTRL
    } else {
      WModifiersState::empty()
    };

    let alt:WModifiersState = if value.alt() {
      WModifiersState::ALT
    } else {
      WModifiersState::empty()
    };

    let logo:WModifiersState = if value.logo() {
      WModifiersState::LOGO
    } else {
      WModifiersState::empty()
    };

    (shift | ctrl | alt | logo).into()
  }
}
