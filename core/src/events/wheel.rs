use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_for_listener, impl_listener,
  impl_listener_and_compose_child, impl_query_self_only, prelude::*,
};

#[derive(Debug, Clone)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: EventCommon,
}

/// Firing the wheel event when the user rotates a wheel button on a pointing
/// device (typically a mouse).
#[derive(Declare)]
pub struct WheelListener {
  #[declare(builtin, convert=custom)]
  on_wheel: MutRefItemSubject<'static, WheelEvent, Infallible>,
}

#[derive(Declare)]
pub struct WheelCaptureListener {
  #[declare(builtin, convert=custom)]
  on_wheel_capture: MutRefItemSubject<'static, WheelEvent, Infallible>,
}

impl_listener_and_compose_child!(
  WheelListener,
  WheelListenerDeclarer,
  on_wheel,
  WheelEvent,
  wheel_stream
);

impl_listener_and_compose_child!(
  WheelCaptureListener,
  WheelCaptureListenerDeclarer,
  on_wheel_capture,
  WheelEvent,
  wheel_capture_stream
);

impl std::borrow::Borrow<EventCommon> for WheelEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for WheelEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for WheelEvent {
  type Target = EventCommon;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for WheelEvent {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::{MockBox, TestWindow};
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

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

    let widget = widget! {
      MockBox {
        size: Size::new(200., 200.),
        on_wheel_capture: move |wheel| {
          *capture_receive.borrow_mut() = (wheel.delta_x,  wheel.delta_y);
          (*capture_event_order.borrow_mut()).push("capture");
        },
        MockBox {
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
      modifiers: ModifiersState::default(),
    });

    assert_eq!(*source_receive_for_bubble.borrow(), (1., 1.));
    assert_eq!(*source_receive_for_capture.borrow(), (1., 1.));
    assert_eq!(*event_order.borrow(), ["capture", "bubble"]);
  }
}
