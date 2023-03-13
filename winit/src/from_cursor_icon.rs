use ribir_core::prelude::CursorIcon as RibirCursorIcon;
use winit::window::CursorIcon as WinitCursorIcon;

pub struct WrappedCursorIcon(WinitCursorIcon);

impl From<WinitCursorIcon> for WrappedCursorIcon {
  fn from(value: WinitCursorIcon) -> Self { WrappedCursorIcon(value) }
}

impl From<WrappedCursorIcon> for WinitCursorIcon {
  fn from(val: WrappedCursorIcon) -> Self { val.0 }
}

impl From<WrappedCursorIcon> for RibirCursorIcon {
  fn from(val: WrappedCursorIcon) -> Self {
    match val.0 {
      WinitCursorIcon::Default => RibirCursorIcon::Default,
      WinitCursorIcon::Crosshair => RibirCursorIcon::Crosshair,
      WinitCursorIcon::Hand => RibirCursorIcon::Hand,
      WinitCursorIcon::Arrow => RibirCursorIcon::Arrow,
      WinitCursorIcon::Move => RibirCursorIcon::Move,
      WinitCursorIcon::Text => RibirCursorIcon::Text,
      WinitCursorIcon::Wait => RibirCursorIcon::Wait,
      WinitCursorIcon::Help => RibirCursorIcon::Help,
      WinitCursorIcon::Progress => RibirCursorIcon::Progress,
      WinitCursorIcon::NotAllowed => RibirCursorIcon::NotAllowed,
      WinitCursorIcon::ContextMenu => RibirCursorIcon::ContextMenu,
      WinitCursorIcon::Cell => RibirCursorIcon::Cell,
      WinitCursorIcon::VerticalText => RibirCursorIcon::VerticalText,
      WinitCursorIcon::Alias => RibirCursorIcon::Alias,
      WinitCursorIcon::Copy => RibirCursorIcon::Copy,
      WinitCursorIcon::NoDrop => RibirCursorIcon::NoDrop,
      WinitCursorIcon::Grab => RibirCursorIcon::Grab,
      WinitCursorIcon::Grabbing => RibirCursorIcon::Grabbing,
      WinitCursorIcon::AllScroll => RibirCursorIcon::AllScroll,
      WinitCursorIcon::ZoomIn => RibirCursorIcon::ZoomIn,
      WinitCursorIcon::ZoomOut => RibirCursorIcon::ZoomOut,
      WinitCursorIcon::EResize => RibirCursorIcon::EResize,
      WinitCursorIcon::NResize => RibirCursorIcon::NResize,
      WinitCursorIcon::NeResize => RibirCursorIcon::NeResize,
      WinitCursorIcon::NwResize => RibirCursorIcon::NwResize,
      WinitCursorIcon::SResize => RibirCursorIcon::SResize,
      WinitCursorIcon::SeResize => RibirCursorIcon::SeResize,
      WinitCursorIcon::SwResize => RibirCursorIcon::SwResize,
      WinitCursorIcon::WResize => RibirCursorIcon::WResize,
      WinitCursorIcon::EwResize => RibirCursorIcon::EwResize,
      WinitCursorIcon::NsResize => RibirCursorIcon::NsResize,
      WinitCursorIcon::NeswResize => RibirCursorIcon::NeswResize,
      WinitCursorIcon::NwseResize => RibirCursorIcon::NwseResize,
      WinitCursorIcon::ColResize => RibirCursorIcon::ColResize,
      WinitCursorIcon::RowResize => RibirCursorIcon::RowResize,
    }
  }
}

impl From<RibirCursorIcon> for WrappedCursorIcon {
  fn from(value: RibirCursorIcon) -> WrappedCursorIcon {
    let w_icon = match value {
      RibirCursorIcon::Default => WinitCursorIcon::Default,
      RibirCursorIcon::Crosshair => WinitCursorIcon::Crosshair,
      RibirCursorIcon::Hand => WinitCursorIcon::Hand,
      RibirCursorIcon::Arrow => WinitCursorIcon::Arrow,
      RibirCursorIcon::Move => WinitCursorIcon::Move,
      RibirCursorIcon::Text => WinitCursorIcon::Text,
      RibirCursorIcon::Wait => WinitCursorIcon::Wait,
      RibirCursorIcon::Help => WinitCursorIcon::Help,
      RibirCursorIcon::Progress => WinitCursorIcon::Progress,
      RibirCursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
      RibirCursorIcon::ContextMenu => WinitCursorIcon::ContextMenu,
      RibirCursorIcon::Cell => WinitCursorIcon::Cell,
      RibirCursorIcon::VerticalText => WinitCursorIcon::VerticalText,
      RibirCursorIcon::Alias => WinitCursorIcon::Alias,
      RibirCursorIcon::Copy => WinitCursorIcon::Copy,
      RibirCursorIcon::NoDrop => WinitCursorIcon::NoDrop,
      RibirCursorIcon::Grab => WinitCursorIcon::Grab,
      RibirCursorIcon::Grabbing => WinitCursorIcon::Grabbing,
      RibirCursorIcon::AllScroll => WinitCursorIcon::AllScroll,
      RibirCursorIcon::ZoomIn => WinitCursorIcon::ZoomIn,
      RibirCursorIcon::ZoomOut => WinitCursorIcon::ZoomOut,
      RibirCursorIcon::EResize => WinitCursorIcon::EResize,
      RibirCursorIcon::NResize => WinitCursorIcon::NResize,
      RibirCursorIcon::NeResize => WinitCursorIcon::NeResize,
      RibirCursorIcon::NwResize => WinitCursorIcon::NwResize,
      RibirCursorIcon::SResize => WinitCursorIcon::SResize,
      RibirCursorIcon::SeResize => WinitCursorIcon::SeResize,
      RibirCursorIcon::SwResize => WinitCursorIcon::SwResize,
      RibirCursorIcon::WResize => WinitCursorIcon::WResize,
      RibirCursorIcon::EwResize => WinitCursorIcon::EwResize,
      RibirCursorIcon::NsResize => WinitCursorIcon::NsResize,
      RibirCursorIcon::NeswResize => WinitCursorIcon::NeswResize,
      RibirCursorIcon::NwseResize => WinitCursorIcon::NwseResize,
      RibirCursorIcon::ColResize => WinitCursorIcon::ColResize,
      RibirCursorIcon::RowResize => WinitCursorIcon::RowResize,
    };
    w_icon.into()
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn from_winit() { let x = WinitCursorIcon::Alias; }
}
