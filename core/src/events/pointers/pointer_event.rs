use super::PointerId;
use crate::{
  prelude::{dispatcher::DispatchInfo, *},
  widget_tree::WidgetTree,
};

impl PointerEvent {
  pub(crate) fn from_mouse(
    pointer_id: Box<dyn PointerId>,
    target: WidgetId,
    tree: &WidgetTree,
    info: &DispatchInfo,
  ) -> Self {
    PointerEvent {
      id: pointer_id,
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
