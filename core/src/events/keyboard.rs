use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_listener_and_compose_child_with_focus, impl_query_self_only, prelude::*,
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

#[derive(Declare)]
pub struct KeyDownCaptureListener {
  #[declare(builtin, convert=custom)]
  on_key_down_capture: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

#[derive(Declare)]
pub struct KeyUpCaptureListener {
  #[declare(builtin, convert=custom)]
  on_key_up_capture: MutRefItemSubject<'static, KeyboardEvent, Infallible>,
}

impl_listener_and_compose_child_with_focus!(
  KeyDownListener,
  KeyDownListenerDeclarer,
  on_key_down,
  KeyboardEvent,
  key_down_stream
);

impl_listener_and_compose_child_with_focus!(
  KeyUpListener,
  KeyUpListenerDeclarer,
  on_key_up,
  KeyboardEvent,
  key_up_stream
);

impl_listener_and_compose_child_with_focus!(
  KeyDownCaptureListener,
  KeyDownCaptureListenerDeclarer,
  on_key_down_capture,
  KeyboardEvent,
  key_down_capture_stream
);

impl_listener_and_compose_child_with_focus!(
  KeyUpCaptureListener,
  KeyUpCaptureListenerDeclarer,
  on_key_up_capture,
  KeyboardEvent,
  key_up_capture_stream
);

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
  use crate::test_helper::*;
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

    #[derive(Default)]
    struct Keys(Rc<RefCell<Vec<String>>>);

    impl Compose for Keys {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: Size::zero(),
            on_key_down_capture: move |key| {
              this.0.borrow_mut().push(format!("key down capture {:?}", key.key));
            },
            on_key_up_capture: move |key| {
              this.0.borrow_mut().push(format!("key up capture {:?}", key.key));
            },
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
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Pressed));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Released));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Pressed));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Released));

    assert_eq!(
      &*keys.borrow(),
      &[
        "key down capture Key0",
        "key down Key0",
        "key up capture Key0",
        "key up Key0",
        "key down capture Key1",
        "key down Key1",
        "key up capture Key1",
        "key up Key1"
      ]
    );
  }
}
