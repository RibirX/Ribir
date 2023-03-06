use ribir_core::window::CursorIcon as CCursorIcon;
use winit::window::CursorIcon as WCursorIcon;

pub struct RCursorIcon(WCursorIcon);

impl From<WCursorIcon> for RCursorIcon {
  fn from(value: WCursorIcon) -> Self { RCursorIcon(value) }
}

impl From<RCursorIcon> for WCursorIcon {
  fn from(val: RCursorIcon) -> Self { val.0 }
}

impl From<RCursorIcon> for CCursorIcon {
  fn from(val: RCursorIcon) -> Self {
    match val.0 {
      WCursorIcon::Default => CCursorIcon::Default,
      WCursorIcon::Crosshair => CCursorIcon::Crosshair,
      WCursorIcon::Hand => CCursorIcon::Hand,
      WCursorIcon::Arrow => CCursorIcon::Arrow,
      WCursorIcon::Move => CCursorIcon::Move,
      WCursorIcon::Text => CCursorIcon::Text,
      WCursorIcon::Wait => CCursorIcon::Wait,
      WCursorIcon::Help => CCursorIcon::Help,
      WCursorIcon::Progress => CCursorIcon::Progress,
      WCursorIcon::NotAllowed => CCursorIcon::NotAllowed,
      WCursorIcon::ContextMenu => CCursorIcon::ContextMenu,
      WCursorIcon::Cell => CCursorIcon::Cell,
      WCursorIcon::VerticalText => CCursorIcon::VerticalText,
      WCursorIcon::Alias => CCursorIcon::Alias,
      WCursorIcon::Copy => CCursorIcon::Copy,
      WCursorIcon::NoDrop => CCursorIcon::NoDrop,
      WCursorIcon::Grab => CCursorIcon::Grab,
      WCursorIcon::Grabbing => CCursorIcon::Grabbing,
      WCursorIcon::AllScroll => CCursorIcon::AllScroll,
      WCursorIcon::ZoomIn => CCursorIcon::ZoomIn,
      WCursorIcon::ZoomOut => CCursorIcon::ZoomOut,
      WCursorIcon::EResize => CCursorIcon::EResize,
      WCursorIcon::NResize => CCursorIcon::NResize,
      WCursorIcon::NeResize => CCursorIcon::NeResize,
      WCursorIcon::NwResize => CCursorIcon::NwResize,
      WCursorIcon::SResize => CCursorIcon::SResize,
      WCursorIcon::SeResize => CCursorIcon::SeResize,
      WCursorIcon::SwResize => CCursorIcon::SwResize,
      WCursorIcon::WResize => CCursorIcon::WResize,
      WCursorIcon::EwResize => CCursorIcon::EwResize,
      WCursorIcon::NsResize => CCursorIcon::NsResize,
      WCursorIcon::NeswResize => CCursorIcon::NeswResize,
      WCursorIcon::NwseResize => CCursorIcon::NwseResize,
      WCursorIcon::ColResize => CCursorIcon::ColResize,
      WCursorIcon::RowResize => CCursorIcon::RowResize,
    }
  }
}

impl From<CCursorIcon> for RCursorIcon {
  fn from(value: CCursorIcon) -> RCursorIcon {
    let w_icon = match value {
      CCursorIcon::Default => WCursorIcon::Default,
      CCursorIcon::Crosshair => WCursorIcon::Crosshair,
      CCursorIcon::Hand => WCursorIcon::Hand,
      CCursorIcon::Arrow => WCursorIcon::Arrow,
      CCursorIcon::Move => WCursorIcon::Move,
      CCursorIcon::Text => WCursorIcon::Text,
      CCursorIcon::Wait => WCursorIcon::Wait,
      CCursorIcon::Help => WCursorIcon::Help,
      CCursorIcon::Progress => WCursorIcon::Progress,
      CCursorIcon::NotAllowed => WCursorIcon::NotAllowed,
      CCursorIcon::ContextMenu => WCursorIcon::ContextMenu,
      CCursorIcon::Cell => WCursorIcon::Cell,
      CCursorIcon::VerticalText => WCursorIcon::VerticalText,
      CCursorIcon::Alias => WCursorIcon::Alias,
      CCursorIcon::Copy => WCursorIcon::Copy,
      CCursorIcon::NoDrop => WCursorIcon::NoDrop,
      CCursorIcon::Grab => WCursorIcon::Grab,
      CCursorIcon::Grabbing => WCursorIcon::Grabbing,
      CCursorIcon::AllScroll => WCursorIcon::AllScroll,
      CCursorIcon::ZoomIn => WCursorIcon::ZoomIn,
      CCursorIcon::ZoomOut => WCursorIcon::ZoomOut,
      CCursorIcon::EResize => WCursorIcon::EResize,
      CCursorIcon::NResize => WCursorIcon::NResize,
      CCursorIcon::NeResize => WCursorIcon::NeResize,
      CCursorIcon::NwResize => WCursorIcon::NwResize,
      CCursorIcon::SResize => WCursorIcon::SResize,
      CCursorIcon::SeResize => WCursorIcon::SeResize,
      CCursorIcon::SwResize => WCursorIcon::SwResize,
      CCursorIcon::WResize => WCursorIcon::WResize,
      CCursorIcon::EwResize => WCursorIcon::EwResize,
      CCursorIcon::NsResize => WCursorIcon::NsResize,
      CCursorIcon::NeswResize => WCursorIcon::NeswResize,
      CCursorIcon::NwseResize => WCursorIcon::NwseResize,
      CCursorIcon::ColResize => WCursorIcon::ColResize,
      CCursorIcon::RowResize => WCursorIcon::RowResize,
    };
    w_icon.into()
  }
}

