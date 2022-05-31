use crate::prelude::*;

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

/// Widget fire event whenever press or release a key.
#[derive(Declare, SingleChild)]
pub struct KeyDownListener {
  #[declare(builtin, custom_convert)]
  pub on_key_down: Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)>,
}

#[derive(Declare, SingleChild)]
pub struct KeyUpListener {
  #[declare(builtin, custom_convert)]
  pub on_key_up: Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)>,
}

impl KeyDownListener {
  #[inline]
  pub fn dispatch_event(&mut self, event: &mut KeyboardEvent) { (self.on_key_down)(event) }
}

impl KeyUpListener {
  #[inline]
  pub fn dispatch_event(&mut self, event: &mut KeyboardEvent) { (self.on_key_up)(event) }
}

impl Render for KeyDownListener {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map(|c| ctx.perform_child_layout(c, clamp))
      .unwrap_or_default()
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Render for KeyUpListener {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map(|c| ctx.perform_child_layout(c, clamp))
      .unwrap_or_default()
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl KeyDownListenerBuilder {
  #[inline]
  pub fn on_key_down_convert(
    f: impl for<'r> FnMut(&'r mut KeyboardEvent) + 'static,
  ) -> Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)> {
    Box::new(f)
  }
}

impl KeyUpListenerBuilder {
  #[inline]
  pub fn on_key_up_convert(
    f: impl for<'r> FnMut(&'r mut KeyboardEvent) + 'static,
  ) -> Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)> {
    Box::new(f)
  }
}

impl std::borrow::Borrow<EventCommon> for KeyboardEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for KeyboardEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for KeyboardEvent {
  type Target = EventCommon;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for KeyboardEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ElementState, KeyboardInput, WindowEvent};

  fn new_key_event(key: VirtualKeyCode, state: ElementState) -> WindowEvent<'static> {
    #[allow(deprecated)]
    WindowEvent::KeyboardInput {
      device_id: unsafe { DeviceId::dummy() },
      input: KeyboardInput {
        scancode: 0,
        virtual_keycode: Some(key),
        state,
        modifiers: ModifiersState::default(),
      },
      is_synthetic: false,
    }
  }

  #[test]
  fn smoke() {
    #[derive(Default)]
    struct Keys(Rc<RefCell<Vec<String>>>);

    impl Compose for Keys {
      fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
        widget! {
          track { this }
          SizedBox {
            size: Size::zero(),
            auto_focus: true,
            on_key_down: move |key| {
              this.0
                .borrow_mut()
                .push(format!("key down {:?}", key.key));
            },
            on_key_up: move |key| {
              this.0.borrow_mut().push(format!("key up {:?}", key.key));
            }
          }
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = Window::without_render(w.into_widget(), Size::new(100., 100.));
    wnd.render_ready();

    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Pressed));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Released));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Pressed));
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Released));

    assert_eq!(
      &*keys.borrow(),
      &[
        "key down Key0",
        "key up Key0",
        "key down Key1",
        "key up Key1"
      ]
    );
  }
}
