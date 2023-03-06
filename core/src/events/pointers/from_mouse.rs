use std::{cell::RefCell, rc::Rc};

use super::PointerId;
use crate::{
  prelude::{dispatcher::DispatchInfo, *},
  widget_tree::WidgetTree,
};
// use winit::event::MouseButton;

impl PointerEvent {
  pub fn from_mouse(
    target: WidgetId,
    tree: &WidgetTree,
    info: Rc<RefCell<dyn DispatchInfo>>,
  ) -> Self {
    PointerEvent {
      // todo: how to generate pointer id ?
      id: PointerId(0),
      width: 1.0,
      height: 1.0,
      pressure: if info.borrow().mouse_button().is_empty() {
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
