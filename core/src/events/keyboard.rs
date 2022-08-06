use crate::{impl_query_self_only, prelude::*};

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

/// Widget fire event whenever press or release a key.
#[derive(Declare)]
pub struct KeyDownListener {
  #[declare(builtin, custom_convert)]
  pub on_key_down: Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)>,
}

#[derive(Declare)]
pub struct KeyUpListener {
  #[declare(builtin, custom_convert)]
  pub on_key_up: Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)>,
}

impl EventListener for KeyDownListener {
  type Event = KeyboardEvent;
  #[inline]
  fn dispatch(&mut self, event: &mut KeyboardEvent) { (self.on_key_down)(event) }
}

impl EventListener for KeyUpListener {
  type Event = KeyboardEvent;
  #[inline]
  fn dispatch(&mut self, event: &mut KeyboardEvent) { (self.on_key_up)(event) }
}

impl ComposeSingleChild for KeyDownListener {
  #[inline]
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    compose_child_as_data_widget(child, this, |w| w)
  }
}

impl ComposeSingleChild for KeyUpListener {
  #[inline]
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    compose_child_as_data_widget(child, this, |w| w)
  }
}

impl Query for KeyDownListener {
  impl_query_self_only!();
}

impl Query for KeyUpListener {
  impl_query_self_only!();
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
    wnd.draw_frame();

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
