use crate::prelude::*;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: EventCommon,
}

/// Firing the wheel event when the user rotates a wheel button on a pointing
/// device (typically a mouse).
#[derive(Default)]
pub struct WheelAttr(LocalSubject<'static, Rc<WheelEvent>, ()>);

impl std::convert::AsRef<EventCommon> for WheelEvent {
  #[inline]
  fn as_ref(&self) -> &EventCommon { self.common.as_ref() }
}

impl std::convert::AsMut<EventCommon> for WheelEvent {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { self.common.as_mut() }
}

impl WheelAttr {
  #[inline]
  pub fn event_observable(&self) -> LocalSubject<'static, Rc<WheelEvent>, ()> { self.0.clone() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new((0., 0.)));
    let c_receive = receive.clone();

    let widget = SizedBox::from_size(Size::new(100., 100.))
      .with_auto_focus(true)
      .on_wheel(move |wheel| {
        *c_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y);
      });
    let mut wnd = window::NoRenderWindow::without_render(widget.box_it(), Size::new(100., 100.));

    wnd.render_ready();
    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::LineDelta(1.0, 1.0),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(*receive.borrow(), (1., 1.));
  }
}
