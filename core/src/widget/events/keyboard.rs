use crate::prelude::*;
use rxrust::prelude::*;
use std::rc::Rc;
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
pub struct KeyboardAttr(LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()>);

pub fn keyboard_listen_on<W: AttachAttr, H: FnMut(&KeyboardEvent) + 'static>(
  widget: W,
  event_type: KeyboardEventType,
  handler: H,
) -> W::W {
  let mut w = widget.into_attr_widget();
  // ensure focus attr attached, because a widget can accept keyboard event base
  // on it can be focused.
  w.attrs_mut().entry::<FocusAttr>().or_default();
  w.attrs_mut()
    .entry::<KeyboardAttr>()
    .or_default()
    .listen_on(event_type, handler);

  w
}

impl KeyboardAttr {
  #[inline]
  pub fn event_observable(
    &self,
  ) -> LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()> {
    self.0.clone()
  }

  pub fn listen_on<H: FnMut(&KeyboardEvent) + 'static>(
    &self,
    event_type: KeyboardEventType,
    mut handler: H,
  ) -> SubscriptionWrapper<MutRc<SingleSubscription>> {
    self
      .event_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event))
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
    let keys = Rc::new(RefCell::new(vec![]));
    let down_keys = keys.clone();
    let up_keys = keys.clone();
    let widget = declare! {
      SizedBox {
        size: Size::zero(),
        auto_focus: true,
        on_key_down: move |key| {
          down_keys
            .borrow_mut()
            .push(format!("key down {:?}", key.key))
        },
        on_key_up: move |key| up_keys.borrow_mut().push(format!("key up {:?}", key.key))
      }
    };

    let mut wnd = window::NoRenderWindow::without_render(widget.box_it(), Size::new(100., 100.));
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
