use crate::{
  impl_all_event, impl_common_event_deref, impl_compose_child_for_listener, impl_listener,
  impl_multi_event_listener, prelude::*, window::WindowId,
};
use rxrust::prelude::*;
use std::convert::Infallible;

#[derive(Debug)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: CommonEvent,
}

pub type WheelSubject = MutRefItemSubject<'static, AllWheel, Infallible>;

impl_multi_event_listener! {
  "The listener use to fire and listen wheel events.",
  Wheel,
  "Firing the wheel event when the user rotates a wheel button on a pointing \
  device (typically a mouse).",
  Wheel,
  "Same as `Wheel` but emit in capture phase.",
  WheelCapture
}
impl_compose_child_for_listener!(WheelListener);
impl_common_event_deref!(WheelEvent);

impl WheelEvent {
  #[inline]
  pub fn new(delta_x: f32, delta_y: f32, id: WidgetId, wnd_id: WindowId) -> Self {
    Self {
      delta_x,
      delta_y,
      common: CommonEvent::new(id, wnd_id),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::{MockBox, TestWindow};
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

  #[test]
  fn smoke() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let source_receive_for_bubble = Rc::new(RefCell::new((0., 0.)));
    let bubble_receive = source_receive_for_bubble.clone();
    let source_receive_for_capture = Rc::new(RefCell::new((0., 0.)));
    let capture_receive = source_receive_for_capture.clone();
    let event_order = Rc::new(RefCell::new(Vec::new()));
    let bubble_event_order = event_order.clone();
    let capture_event_order = event_order.clone();

    let widget = fn_widget! {
      @MockBox {
        size: Size::new(200., 200.),
        on_wheel_capture: move |wheel| {
          *capture_receive.borrow_mut() = (wheel.delta_x,  wheel.delta_y);
          (*capture_event_order.borrow_mut()).push("capture");
        },
        @MockBox {
          size: Size::new(100., 100.),
          auto_focus: true,
          on_wheel: move |wheel| {
            *bubble_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y);
            (*bubble_event_order.borrow_mut()).push("bubble");
          }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(widget, Size::new(100., 100.));

    wnd.draw_frame();
    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta((1.0, 1.0).into()),
      phase: TouchPhase::Started,
    });
    wnd.run_frame_tasks();

    assert_eq!(*source_receive_for_bubble.borrow(), (1., 1.));
    assert_eq!(*source_receive_for_capture.borrow(), (1., 1.));
    assert_eq!(*event_order.borrow(), ["capture", "bubble"]);
  }
}
