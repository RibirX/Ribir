use crate::{
  impl_all_event, impl_common_event_deref, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_multi_event_listener, impl_query_self_only, prelude::*,
};
use rxrust::{
  prelude::*,
  rc::{MutRc, RcDeref, RcDerefMut},
};
use smallvec::SmallVec;
use std::convert::Infallible;

#[derive(Debug)]
pub struct CharsEvent<'a> {
  pub chars: String,
  pub common: CommonEvent<'a>,
}

impl_event_subject!(Chars, event_name = AllChars);

impl_multi_event_listener! {
  "The listener use to fire and listen chars events.",
  Chars,
  "", Chars,
  "", CharsCapture
}

impl_compose_child_with_focus_for_listener!(CharsListener);

impl_common_event_deref!(CharsEvent);

impl<'a> CharsEvent<'a> {
  #[inline]
  pub fn new(chars: String, id: WidgetId, wnd: &'a Window) -> Self {
    Self {
      chars,
      common: CommonEvent::new(id, wnd),
    }
  }
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
