use std::cell::RefCell;

use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

#[derive(Debug, Clone)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: EventCommon,
}

type WhellCallback = dyn for<'r> FnMut(&'r mut WheelEvent);
/// Firing the wheel event when the user rotates a wheel button on a pointing
/// device (typically a mouse).
#[derive(Declare)]
pub struct WheelListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(&'r mut WheelEvent), wrap_fn = RefCell::new))]
  wheel: RefCell<Box<WhellCallback>>,
}

impl ComposeChild for WheelListener {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for WheelListener {
  impl_query_self_only!();
}

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

impl EventListener for WheelListener {
  type Event = WheelEvent;
  #[inline]
  fn dispatch(&self, event: &mut WheelEvent) { (self.wheel.borrow_mut())(event) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::MockBox;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new((0., 0.)));
    let c_receive = receive.clone();

    let widget = widget! {
      MockBox {
        size: Size::new(100., 100.),
        auto_focus: true,
        wheel: move |wheel| *c_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y)
      }
    };

    let mut wnd = Window::default_mock(widget, Some(Size::new(100., 100.)));

    wnd.draw_frame();
    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::PixelDelta((1.0, 1.0).into()),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(*receive.borrow(), (1., 1.));
  }
}
