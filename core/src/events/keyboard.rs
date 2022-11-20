use std::cell::RefCell;

use crate::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

type Callback = RefCell<Box<dyn for<'r> FnMut(&'r mut KeyboardEvent)>>;

/// Widget fire event whenever press or release a key.
#[derive(Declare)]
pub struct KeyDownListener {
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut KeyboardEvent),
    wrap_fn = RefCell::new)
  )]
  pub key_down: Callback,
}

#[derive(Declare)]
pub struct KeyUpListener {
  #[declare(
    builtin,
    convert=box_trait(for<'r> FnMut(&'r mut KeyboardEvent),
    wrap_fn = RefCell::new)
  )]
  pub key_up: Callback,
}

impl EventListener for KeyDownListener {
  type Event = KeyboardEvent;
  #[inline]
  fn dispatch(&self, event: &mut KeyboardEvent) { (self.key_down.borrow_mut())(event) }
}

impl EventListener for KeyUpListener {
  type Event = KeyboardEvent;
  #[inline]
  fn dispatch(&self, event: &mut KeyboardEvent) { (self.key_up.borrow_mut())(event) }
}

impl ComposeChild for KeyDownListener {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl ComposeChild for KeyUpListener {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for KeyDownListener {
  impl_query_self_only!();
}

impl Query for KeyUpListener {
  impl_query_self_only!();
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
      fn compose(this: StateWidget<Self>) -> Widget {
        widget! {
          track { this: this.into_stateful() }
          MockBox {
            size: Size::zero(),
            auto_focus: true,
            key_down: move |key| {
              this.0
                .borrow_mut()
                .push(format!("key down {:?}", key.key));
            },
            key_up: move |key| {
              this.0.borrow_mut().push(format!("key up {:?}", key.key));
            }
          }
        }
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
