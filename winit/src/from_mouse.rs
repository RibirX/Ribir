use winit::event::MouseButton as WinitMouseButton;

use ribir_core::events::MouseButtons as RibirMouseButton;

pub struct WrappedMouseButton(WinitMouseButton);

impl From<WinitMouseButton> for WrappedMouseButton {
  fn from(value: WinitMouseButton) -> Self { WrappedMouseButton(value) }
}

impl From<WrappedMouseButton> for WinitMouseButton {
  fn from(val: WrappedMouseButton) -> Self { val.0 }
}

impl From<RibirMouseButton> for WrappedMouseButton {
  fn from(value: RibirMouseButton) -> Self {
    match value {
      RibirMouseButton::PRIMARY => WrappedMouseButton(WinitMouseButton::Left),
      RibirMouseButton::SECONDARY => WrappedMouseButton(WinitMouseButton::Right),
      RibirMouseButton::AUXILIARY => WrappedMouseButton(WinitMouseButton::Middle),
      RibirMouseButton::FOURTH => WrappedMouseButton(WinitMouseButton::Other(1)),
      RibirMouseButton::FIFTH => WrappedMouseButton(WinitMouseButton::Other(2)),
      v => {
        log::warn!("The mouse button {v:?} is not supported.");
        WrappedMouseButton(WinitMouseButton::Other(0))
      }
    }
  }
}

impl From<WrappedMouseButton> for RibirMouseButton {
  fn from(val: WrappedMouseButton) -> Self {
    match val.0 {
      WinitMouseButton::Left => RibirMouseButton::PRIMARY,
      WinitMouseButton::Right => RibirMouseButton::SECONDARY,
      WinitMouseButton::Middle => RibirMouseButton::AUXILIARY,
      WinitMouseButton::Other(1) => RibirMouseButton::FOURTH,
      WinitMouseButton::Other(2) => RibirMouseButton::FIFTH,
      WinitMouseButton::Other(v) => {
        log::warn!("The mouse button {} is not supported.", v);
        RibirMouseButton::default()
      }
    }
  }
}
