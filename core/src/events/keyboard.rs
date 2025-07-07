pub use winit::keyboard::{
  Key as VirtualKey, KeyCode, KeyLocation, ModifiersState, NamedKey, PhysicalKey,
};

use crate::{impl_common_event_deref, prelude::*};

#[derive(Debug)]
pub struct KeyboardEvent {
  physical_key: PhysicalKey,
  key: VirtualKey,
  is_repeat: bool,
  location: KeyLocation,
  common: CommonEvent,
}

impl KeyboardEvent {
  #[inline]
  pub fn key_code(&self) -> &PhysicalKey { &self.physical_key }

  #[inline]
  pub fn key(&self) -> &VirtualKey { &self.key }

  #[inline]
  pub fn is_repeat(&self) -> bool { self.is_repeat }

  #[inline]
  pub fn location(&self) -> KeyLocation { self.location }
}

impl_common_event_deref!(KeyboardEvent);

impl KeyboardEvent {
  #[inline]
  pub fn new(
    wnd: &Window, id: WidgetId, physical_key: PhysicalKey, key: VirtualKey, is_repeat: bool,
    location: KeyLocation,
  ) -> Self {
    Self { physical_key, key, is_repeat, location, common: CommonEvent::new(id, wnd.tree) }
  }
}

#[cfg(test)]
mod tests {

  use winit::event::ElementState;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    struct Keys(Stateful<Vec<String>>);

    impl Compose for Keys {
      fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
            on_key_down_capture: move |key| {
              $write(this).0.write().push(format!("key down capture {:?}", key.key));
            },
            on_key_up_capture: move |key| {
              $write(this).0.write().push(format!("key up capture {:?}", key.key));
            },
            @MockBox {
              size: Size::zero(),
              auto_focus: true,
              on_key_down: move |key| {
                $write(this).0.write().push(format!("key down {:?}", key.key));
              },
              on_key_up: move |key| {
                $write(this).0.write().push(format!("key up {:?}", key.key));
              }
            }
          }
        }
        .into_widget()
      }
    }

    let keys = Stateful::new(vec![]);
    let k2 = keys.clone_reader();

    let wnd = TestWindow::from_widget(fn_widget! {
      Keys(keys.clone_writer())
    });
    wnd.draw_frame();

    wnd.process_keyboard_event(
      PhysicalKey::Code(KeyCode::Digit0),
      VirtualKey::Character("0".into()),
      false,
      KeyLocation::Standard,
      ElementState::Pressed,
    );

    wnd.process_keyboard_event(
      PhysicalKey::Code(KeyCode::Digit0),
      VirtualKey::Character("0".into()),
      false,
      KeyLocation::Standard,
      ElementState::Released,
    );

    wnd.process_keyboard_event(
      PhysicalKey::Code(KeyCode::Digit1),
      VirtualKey::Character("1".into()),
      false,
      KeyLocation::Standard,
      ElementState::Pressed,
    );

    wnd.process_keyboard_event(
      PhysicalKey::Code(KeyCode::Digit1),
      VirtualKey::Character("1".into()),
      false,
      KeyLocation::Standard,
      ElementState::Released,
    );

    wnd.run_frame_tasks();
    assert_eq!(
      &*k2.read(),
      &[
        "key down capture Character(\"0\")",
        "key down Character(\"0\")",
        "key up capture Character(\"0\")",
        "key up Character(\"0\")",
        "key down capture Character(\"1\")",
        "key down Character(\"1\")",
        "key up capture Character(\"1\")",
        "key up Character(\"1\")"
      ]
    );
  }
}
