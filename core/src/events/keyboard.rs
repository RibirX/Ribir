use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_query_self_only, prelude::*,
};

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

#[derive(Declare)]
pub struct KeyDownListener {
  #[declare(builtin, default, convert=custom)]
  on_key_down: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

#[derive(Declare)]
pub struct KeyUpListener {
  #[declare(
    builtin,
    convert=custom
  )]
  on_key_up: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

impl_listener!(
  KeyDownListener,
  KeyDownListenerDeclarer,
  on_key_down,
  KeyboardEvent,
  key_down_stream
);
impl_compose_child_with_focus_for_listener!(KeyDownListener);

impl_listener!(
  KeyUpListener,
  KeyUpListenerDeclarer,
  on_key_up,
  KeyboardEvent,
  key_up_stream
);

impl_compose_child_with_focus_for_listener!(KeyUpListener);

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
  use crate::test::*;
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
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
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
        .into_widget()
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = Window::default_mock(w.into_widget(), None);
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
