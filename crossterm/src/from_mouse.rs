use crossterm::event::MouseButton as CrosstermMouseButton;
use ribir_core::events::MouseButtons as RibirMouseButton;

pub struct WrappedMouseButton(CrosstermMouseButton);

impl From<CrosstermMouseButton> for WrappedMouseButton {
  fn from(value: CrosstermMouseButton) -> Self { WrappedMouseButton(value) }
}

impl From<WrappedMouseButton> for RibirMouseButton {
  fn from(val: WrappedMouseButton) -> Self {
    match val.0 {
      CrosstermMouseButton::Left => RibirMouseButton::PRIMARY,
      CrosstermMouseButton::Right => RibirMouseButton::SECONDARY,
      CrosstermMouseButton::Middle => RibirMouseButton::AUXILIARY,
    }
  }
}
