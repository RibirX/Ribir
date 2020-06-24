use super::{MouseButtons, PointerEvent, PointerId, PointerType};
use crate::{
  events::{EventCommon, ModifiersState},
  prelude::*,
  widget::widget_tree::WidgetId,
};
use winit::event::MouseButton;

impl PointerEvent {
  pub fn from_mouse(
    target: WidgetId,
    position: Point,
    global_pos: Point,
    modifiers: ModifiersState,
    btn: MouseButtons,
  ) -> Self {
    let event = EventCommon {
      target,
      current_target: target,
      composed_path: vec![],
      cancel_bubble: <_>::default(),
      modifiers,
    };

    PointerEvent {
      position,
      global_pos,
      // todo: how to generate pointer id ?
      id: PointerId(0),
      width: 1.0,
      height: 1.0,
      pressure: 0.5,
      tilt_x: 90.,
      tilt_y: 90.,
      twist: 0.,
      point_type: PointerType::Mouse,
      is_primary: true,
      buttons: btn.into(),
      common: event,
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
