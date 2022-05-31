use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WheelEvent {
  pub delta_x: f32,
  pub delta_y: f32,
  pub common: EventCommon,
}

/// Firing the wheel event when the user rotates a wheel button on a pointing
/// device (typically a mouse).

#[derive(Declare, SingleChild)]
pub struct WheelListener {
  #[declare(builtin, custom_convert)]
  on_wheel: Box<dyn for<'r> FnMut(&'r mut WheelEvent)>,
}

impl Render for WheelListener {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map(|c| ctx.perform_child_layout(c, clamp))
      .unwrap_or_default()
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl WheelListenerBuilder {
  #[inline]
  pub fn on_wheel_convert(
    f: impl for<'r> FnMut(&'r mut WheelEvent) + 'static,
  ) -> Box<dyn for<'r> FnMut(&'r mut WheelEvent)> {
    Box::new(f)
  }
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

impl WheelListener {
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

    let widget = widget! {
      SizedBox {
        SizedBox {
          size: Size::new(100., 100.),
          auto_focus: true,
          on_wheel: move |wheel| *c_receive.borrow_mut() = (wheel.delta_x, wheel.delta_y)
        }
      }
    };

    let mut wnd = Window::without_render(widget, Size::new(100., 100.));

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
