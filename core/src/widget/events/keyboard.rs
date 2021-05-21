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

#[derive(Debug, Default)]
pub struct KeyboardAttr(LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()>);

/// A widget that fire event whenever press or release a key.
pub type KeyboardListener<W> = AttrWidget<W, KeyboardAttr>;

impl<W: Widget> KeyboardListener<W> {
  pub fn from_widget<A: AttachAttr<W = W>>(widget: A) -> Self {
    let (major, mut others, widget) = widget.take_attr();

    let major = major.unwrap_or_else(|| {
      let other_attrs = others.get_or_insert_with(<_>::default);
      if other_attrs.find_attr::<FocusAttr>().is_none() {
        other_attrs.front_push_attr(FocusAttr::default());
      }

      KeyboardAttr::default()
    });

    KeyboardListener { major, others, widget }
  }

  #[inline]
  pub fn event_observable(
    &self,
  ) -> LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()> {
    self.major.event_observable()
  }

  pub fn listen_on<H: FnMut(&KeyboardEvent) + 'static>(
    &self,
    event_type: KeyboardEventType,
    mut handler: H,
  ) -> SubscriptionWrapper<LocalSubscription> {
    self
      .event_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event))
  }
}

impl KeyboardAttr {
  #[inline]
  pub fn event_observable(
    &self,
  ) -> LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()> {
    self.0.clone()
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
    let widget = SizedBox::empty_box(Size::zero())
      .with_auto_focus(true)
      .on_key_down(move |key| {
        down_keys
          .borrow_mut()
          .push(format!("key down {:?}", key.key))
      })
      .on_key_up(move |key| up_keys.borrow_mut().push(format!("key up {:?}", key.key)));
    let mut wnd = window::NoRenderWindow::without_render(widget, Size::new(100., 100.));
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
