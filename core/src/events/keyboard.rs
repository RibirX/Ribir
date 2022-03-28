use crate::prelude::*;
use rxrust::prelude::*;
use std::ptr::NonNull;
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum KeyboardEventType {
  KeyDown,
  KeyUp,
}

#[derive(Debug)]
pub struct KeyboardEvent {
  pub scan_code: ScanCode,
  pub key: VirtualKeyCode,
  pub common: EventCommon,
}

/// An attributes that fire event whenever press or release a key.
#[derive(Default)]
pub struct KeyboardAttr(LocalSubject<'static, (KeyboardEventType, NonNull<KeyboardEvent>), ()>);

impl KeyboardAttr {
  #[inline]
  pub fn dispatch_event(&self, event_type: KeyboardEventType, event: &mut KeyboardEvent) {
    self.0.clone().next((event_type, NonNull::from(event)))
  }

  pub fn listen_on<H: FnMut(&KeyboardEvent) + 'static>(
    &self,
    event_type: KeyboardEventType,
    mut handler: H,
  ) -> SubscriptionWrapper<MutRc<SingleSubscription>> {
    self
      .0
      .clone()
      .filter(move |(t, _)| *t == event_type)
      // Safety: Inner pointer from a mut reference and pass to handler one by one.
      .subscribe(move |(_, mut event)| handler(unsafe { event.as_mut() }))
  }
}

impl std::convert::AsRef<EventCommon> for KeyboardEvent {
  #[inline]
  fn as_ref(&self) -> &EventCommon { &self.common }
}

impl std::convert::AsMut<EventCommon> for KeyboardEvent {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { &mut self.common }
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

    impl CombinationWidget for Keys {
      #[widget]
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let down_keys = self.0.clone();
        let up_keys = self.0.clone();
        widget! {
          declare SizedBox {
            size: Size::zero(),
            auto_focus: true,
            on_key_down: move |key| {
              down_keys
                .borrow_mut()
                .push(format!("key down {:?}", key.key))
            },
            on_key_up: move |key| up_keys.borrow_mut().push(format!("key up {:?}", key.key))
          }
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = Window::without_render(w.box_it(), Size::new(100., 100.));
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
