use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_query_self_only, prelude::*,
};

/// An attribute that sends a single Unicode codepoint. The character can be
/// pushed to the end of a string.
#[derive(Declare)]
pub struct CharListener {
  #[declare(builtin, convert=custom)]
  on_char: MutRefItemSubject<'static, CharEvent, ()>,
}

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

impl_listener!(
  CharListener,
  CharListenerDeclarer,
  on_char,
  CharEvent,
  char_stream
);
impl_compose_child_with_focus_for_listener!(CharListener);

impl std::borrow::Borrow<EventCommon> for CharEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for CharEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for CharEvent {
  type Target = EventCommon;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for CharEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = widget! {
      MockBox {
        size: ZERO_SIZE,
        auto_focus: true,
        on_char: move |key| c_receive.borrow_mut().push(key.char)
      }
    };
    let mut wnd = Window::default_mock(widget.into_widget(), None);

    let test_text_case = "Hello 世界！";
    wnd.draw_frame();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
