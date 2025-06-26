use crate::{impl_common_event_deref, prelude::*};

#[derive(Debug)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: CommonEvent,
}

impl_common_event_deref!(WheelEvent);

impl WheelEvent {
  #[inline]
  pub fn new(delta_x: f32, delta_y: f32, id: WidgetId, wnd: &Window) -> Self {
    Self { delta_x, delta_y, common: CommonEvent::new(id, wnd.tree) }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    let (bubble_receive_reader, bubble_receive) = split_value((0., 0.));
    let (capture_receive_reader, capture_receive) = split_value((0., 0.));
    let (event_order_reader, event_order) = split_value(vec![]);

    let widget = fn_widget! {
      @MockBox {
        size: Size::new(200., 200.),
        on_wheel_capture: move |wheel| {
          *$capture_receive.write() = (wheel.delta_x,  wheel.delta_y);
          $event_order.write().push("capture");
        },
        @MockBox {
          size: Size::new(100., 100.),
          auto_focus: true,
          on_wheel: move |wheel| {
            *$bubble_receive.write() = (wheel.delta_x, wheel.delta_y);
            $event_order.write().push("bubble");
          }
        }
      }
    };

    let wnd = TestWindow::new_with_size(widget, Size::new(100., 100.));

    wnd.draw_frame();

    wnd.process_wheel(1.0, 1.0);
    wnd.run_frame_tasks();

    assert_eq!(*bubble_receive_reader.read(), (1., 1.));
    assert_eq!(*capture_receive_reader.read(), (1., 1.));
    assert_eq!(*event_order_reader.read(), ["capture", "bubble"]);
  }
}
