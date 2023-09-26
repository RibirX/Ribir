use rxrust::prelude::*;
use std::convert::Infallible;

use crate::{
  impl_all_event, impl_common_event_deref, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_multi_event_listener, prelude::*, window::WindowId,
};

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  common: CommonEvent,
}

pub type KeyboardSubject = MutRefItemSubject<'static, AllKeyboard, Infallible>;

impl_multi_event_listener! {
  "The listener use to fire and listen keyboard events.",
  Keyboard,
  "The `KeyDown` event is fired when a key is pressed.",
  KeyDown,
  "The `KeyDownCapture` event is same as `KeyDown` but emit in capture phase.",
  KeyDownCapture,
  "The `KeyUp` event is fired when a key is released.",
  KeyUp,
  "The `KeyUpCapture` event is same as `KeyUp` but emit in capture phase.",
  KeyUpCapture
}

impl_common_event_deref!(KeyboardEvent);

impl_compose_child_with_focus_for_listener!(KeyboardListener);

impl KeyboardEvent {
  #[inline]
  pub fn new(scan_code: ScanCode, key: VirtualKeyCode, id: WidgetId, wnd_id: WindowId) -> Self {
    Self {
      scan_code,
      key,
      common: CommonEvent::new(id, wnd_id),
    }
  }
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
      fn compose(this: State<Self>) -> impl WidgetBuilder {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
            on_key_down_capture: move |key| {
              $this.0.borrow_mut().push(format!("key down capture {:?}", key.key));
            },
            on_key_up_capture: move |key| {
              $this.0.borrow_mut().push(format!("key up capture {:?}", key.key));
            },
            @MockBox {
              size: Size::zero(),
              auto_focus: true,
              on_key_down: move |key| {
                $this.0
                  .borrow_mut()
                  .push(format!("key down {:?}", key.key));
              },
              on_key_up: move |key| {
                $this.0.borrow_mut().push(format!("key up {:?}", key.key));
              }
            }
          }
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = TestWindow::new(fn_widget!(w));
    wnd.draw_frame();

    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Pressed));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key0, ElementState::Released));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Pressed));
    #[allow(deprecated)]
    wnd.processes_native_event(new_key_event(VirtualKeyCode::Key1, ElementState::Released));

    wnd.run_frame_tasks();

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
