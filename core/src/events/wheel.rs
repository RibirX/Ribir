use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: EventCommon,
}

/// Firing the wheel event when the user rotates a wheel button on a pointing
/// device (typically a mouse).

#[derive(Declare)]
pub struct WheelListener<F: for<'r> FnMut(&'r mut WheelEvent)> {
  #[declare(builtin)]
  on_wheel: F,
}

impl<F: for<'r> FnMut(&'r mut WheelEvent) + 'static> IntoWidget for WheelListener<F> {
  type W = WheelListener<Box<dyn for<'r> FnMut(&'r mut WheelEvent)>>;

  #[inline]

  fn into_widget(self) -> Self::W { WheelListener { on_wheel: Box::new(self.on_wheel) } }
}

impl std::convert::AsRef<EventCommon> for WheelEvent {
  #[inline]
  fn as_ref(&self) -> &EventCommon { self.common.as_ref() }
}

impl std::convert::AsMut<EventCommon> for WheelEvent {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { self.common.as_mut() }
}

impl WheelListener<Box<dyn for<'r> FnMut(&'r mut WheelEvent)>> {
  #[inline]
  pub fn dispatch_event(&mut self, event: &mut WheelEvent) { (self.on_wheel)(event) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new((0., 0.)));
    let c_receive = receive.clone();

    let widget = SizedBox { size: Size::new(100., 100.) }
      .with_auto_focus(true)
      .on_wheel(move |wheel| {
        *c_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y);
      })
      .box_it();

    let mut wnd = Window::without_render(widget.box_it(), Size::new(100., 100.));

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
