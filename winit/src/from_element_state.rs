use ribir_core::prelude::ElementState as RibirElementState;
use winit::event::ElementState as WinitElementState;

pub struct WrappedElementState(WinitElementState);

impl From<WinitElementState> for WrappedElementState {
  fn from(value: WinitElementState) -> Self { WrappedElementState(value) }
}

impl From<WrappedElementState> for RibirElementState {
  fn from(val: WrappedElementState) -> Self {
    match val.0 {
      WinitElementState::Pressed => RibirElementState::Pressed,
      WinitElementState::Released => RibirElementState::Released,
    }
  }
}
