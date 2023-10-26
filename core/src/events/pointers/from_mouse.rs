use super::PointerId;
use crate::prelude::*;
use winit::event::MouseButton;

impl PointerEvent {
  pub(crate) fn from_mouse(target: WidgetId, wnd: &Window) -> Self {
    let no_button = wnd.dispatcher.borrow().info.mouse_buttons().is_empty();
    PointerEvent {
      // todo: we need to trace the pressed pointer, how to generate pointer id, by device + button?
      id: PointerId(0),
      width: 1.0,
      height: 1.0,
      pressure: if no_button { 0. } else { 0.5 },
      tilt_x: 90.,
      tilt_y: 90.,
      twist: 0.,
      point_type: PointerType::Mouse,
      is_primary: true,
      common: CommonEvent::new(target, wnd.id()),
    }
  }
}

impl From<MouseButton> for MouseButtons {
  fn from(btns: MouseButton) -> Self {
    match btns {
      MouseButton::Left => MouseButtons::PRIMARY,
      MouseButton::Right => MouseButtons::SECONDARY,
      MouseButton::Middle => MouseButtons::AUXILIARY,
      MouseButton::Back => MouseButtons::FOURTH,
      MouseButton::Forward => MouseButtons::FIFTH,
      MouseButton::Other(v) => {
        log::warn!("Not support the mouse button {} now", v);
        MouseButtons::default()
      }
    }
  }
}
