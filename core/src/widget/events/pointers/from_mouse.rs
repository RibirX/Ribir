use super::{
  events::{EventCommon, ModifiersState},
  MouseButtons, PointerEvent, PointerId, PointerType,
};
use crate::prelude::*;
use std::{cell::RefCell, rc::Rc};
use window::RawWindow;
use winit::event::MouseButton;

impl PointerEvent {
  pub(crate) fn from_mouse(
    target: WidgetId,
    position: Point,
    global_pos: Point,
    modifiers: ModifiersState,
    btn: MouseButtons,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Self {
    let event = EventCommon {
      modifiers,
      target,
      current_target: target,
      cancel_bubble: <_>::default(),
      window,
    };

    PointerEvent {
      position,
      global_pos,
      // todo: how to generate pointer id ?
      id: PointerId(0),
      width: 1.0,
      height: 1.0,
      pressure: if btn.is_empty() { 0. } else { 0.5 },
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
