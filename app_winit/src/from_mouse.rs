use winit::event::MouseButton;

use ribir_core::events::MouseButtons;

pub struct RMouseButton(MouseButton);

impl From<MouseButton> for RMouseButton {
  fn from(value: MouseButton) -> Self { RMouseButton(value) }
}

impl From<RMouseButton> for MouseButton {
  fn from(value: RMouseButton) -> Self { value.0 }
}

impl From<RMouseButton> for MouseButtons {
  fn from(val: RMouseButton) -> Self {
    match val.0 {
      MouseButton::Left => MouseButtons::PRIMARY,
      MouseButton::Right => MouseButtons::SECONDARY,
      MouseButton::Middle => MouseButtons::AUXILIARY,
      MouseButton::Other(1) => MouseButtons::FOURTH,
      MouseButton::Other(2) => MouseButtons::FIFTH,
      MouseButton::Other(v) => {
        log::warn!("The mouse button {} is not supported.", v);
        MouseButtons::default()
      }
    }
  }
}
