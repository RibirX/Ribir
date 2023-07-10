use std::convert::Infallible;

use crate::{
  data_widget::compose_child_as_data_widget, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_listener_and_compose_child_with_focus, impl_query_self_only, prelude::*,
};

/// An attribute that sends a single Unicode codepoint. The character can be
/// pushed to the end of a string.
#[derive(Declare)]
pub struct CharsListener {
  #[declare(builtin, convert=custom)]
  on_chars: MutRefItemSubject<'static, CharsEvent, Infallible>,
}

#[derive(Debug)]
pub struct CharsEvent {
  pub chars: String,
  pub common: EventCommon,
}

impl_listener_and_compose_child_with_focus!(
  CharsListener,
  CharsListenerDeclarer,
  on_chars,
  CharsEvent,
  chars_stream
);

#[derive(Declare)]
pub struct CharsCaptureListener {
  #[declare(builtin, convert=custom)]
  on_chars_capture: MutRefItemSubject<'static, CharsEvent, Infallible>,
}

impl_listener_and_compose_child_with_focus!(
  CharsCaptureListener,
  CharsCaptureListenerDeclarer,
  on_chars_capture,
  CharsEvent,
  chars_stream_capture
);

impl std::borrow::Borrow<EventCommon> for CharsEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for CharsEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for CharsEvent {
  type Target = EventCommon;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for CharsEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;

  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;

  #[test]
  fn smoke() {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = widget! {
      MockBox {
        size: ZERO_SIZE,
        auto_focus: true,
        on_chars: move |event| c_receive.borrow_mut().push_str(&event.chars)
      }
    };
    let mut wnd = TestWindow::new(widget);

    let test_text_case = "Hello 世界！";
    wnd.draw_frame();
    #[allow(deprecated)]
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }

  #[test]
  fn chars_capture() {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    let receive = Rc::new(RefCell::new("".to_string()));
    let chars_receive = receive.clone();
    let capture_receive = receive.clone();

    let widget = widget! {
      MockBox {
        size: ZERO_SIZE,
        on_chars_capture: move |event| {
          let chars = event.chars.to_string();
          // The value received first is multiplied by 2
          let char = (chars.parse::<i32>().unwrap() * 2).to_string();
          capture_receive.borrow_mut().push_str(&char);
        },
        MockBox {
          size: ZERO_SIZE,
          auto_focus: true,
          on_chars: move |event| chars_receive.borrow_mut().push_str(&event.chars),
        }
      }
    };
    let mut wnd = TestWindow::new(widget);

    let test_text_case = "123";
    wnd.draw_frame();
    #[allow(deprecated)]
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), "214263");
  }
}
