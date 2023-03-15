use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_for_listener, impl_listener,
  impl_query_self_only, prelude::*,
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

impl_listener!(
  WheelListener,
  WheelListenerDeclarer,
  on_wheel,
  WheelEvent,
  wheel_stream
);

impl_compose_child_for_listener!(WheelListener);

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
  use crate::test::MockBox;
  use std::{cell::RefCell, rc::Rc};
  // use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase,
  // WindowEvent};

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new((0., 0.)));
    let c_receive = receive.clone();

    let widget = widget! {
      MockBox {
        size: Size::new(100., 100.),
        auto_focus: true,
        on_wheel: move |wheel| *c_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y)
      }
    };

    let mut wnd = Window::default_mock(widget, Some(Size::new(100., 100.)));

    wnd.draw_frame();
    let device_id = MockPointerId::zero();
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta(DeviceOffset::new(1, 1)),
      phase: TouchPhase::Started,
    });

    assert_eq!(*receive.borrow(), (1., 1.));
  }
}
