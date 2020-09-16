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

/// A widget that fire event whenever press or release a key.
#[derive(Debug)]
pub struct KeyboardListener {
  widget: BoxWidget,
  subject: LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()>,
}

widget::inherit_widget!(KeyboardListener, widget);

impl KeyboardListener {
  pub fn from_widget(widget: BoxWidget) -> BoxWidget {
    widget::inherit(
      widget.box_it(),
      |base| Self {
        widget: base,
        subject: <_>::default(),
      },
      |_| {},
    )
  }

  #[inline]
  pub fn event_observable(
    &self,
  ) -> LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()> {
    self.subject.clone()
  }

  pub fn listen_on<H: FnMut(&KeyboardEvent) + 'static>(
    base: BoxWidget,
    event_type: KeyboardEventType,
    mut handler: H,
  ) -> BoxWidget {
    let keyboard = Self::from_widget(base);
    Widget::dynamic_cast_ref::<Self>(&keyboard)
      .unwrap()
      .event_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event));
    keyboard
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
