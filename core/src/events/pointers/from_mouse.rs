use super::PointerId;
use crate::prelude::{dispatcher::DispatchInfo, widget_tree::WidgetTree, *};
use winit::event::MouseButton;

impl PointerEvent {
  pub(crate) fn from_mouse(target: WidgetId, tree: &WidgetTree, info: &mut DispatchInfo) -> Self {
    PointerEvent {
      // todo: how to generate pointer id ?
      id: PointerId(0),
      width: 1.0,
      height: 1.0,
      pressure: if info.mouse_buttons().is_empty() {
        0.
      } else {
        0.5
      },
      tilt_x: 90.,
      tilt_y: 90.,
      twist: 0.,
      point_type: PointerType::Mouse,
      is_primary: true,
      common: EventCommon::new(target, tree, info),
    }
  }
}

impl From<MouseButton> for MouseButtons {
  fn from(btns: MouseButton) -> Self {
    match btns {
      MouseButton::Left => MouseButtons::PRIMARY,
      MouseButton::Right => MouseButtons::SECONDARY,
      MouseButton::Middle => MouseButtons::AUXILIARY,
      MouseButton::Other(1) => MouseButtons::FOURTH,
      MouseButton::Other(2) => MouseButtons::FIFTH,
      MouseButton::Other(v) => {
        log::warn!("Not support the mouse button {} now", v);
        MouseButtons::default()
      }
    }
  }
}
