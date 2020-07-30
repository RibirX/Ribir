use super::{
  events::{EventCommon, ModifiersState},
  MouseButtons, PointerEvent, PointerId, PointerType,
};
use crate::{prelude::*, widget::widget_tree::WidgetId};
use winit::event::MouseButton;

impl PointerEvent {
  pub(crate) fn from_mouse_without_target(
    global_pos: Point,
    modifiers: ModifiersState,
    btn: MouseButtons,
  ) -> Self {
    let target = uninit_target();
    let event = EventCommon {
      target,
      current_target: target,
      composed_path: vec![],
      cancel_bubble: <_>::default(),
      modifiers,
    };

    PointerEvent {
      position: global_pos,
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
      buttons: btn,
      common: event,
    }
  }
}
fn uninit_target() -> WidgetId {
  let id = std::num::NonZeroUsize::new(0);
  unsafe { std::mem::transmute(id) }
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
