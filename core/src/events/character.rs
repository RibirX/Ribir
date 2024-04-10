use crate::{impl_common_event_deref, prelude::*, window::WindowId};

#[derive(Debug)]
pub struct CharsEvent {
  pub chars: String,
  pub common: CommonEvent,
}

impl_common_event_deref!(CharsEvent);

impl CharsEvent {
  #[inline]
  pub fn new(chars: String, id: WidgetId, wnd_id: WindowId) -> Self {
    Self { chars, common: CommonEvent::new(id, wnd_id) }
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use super::*;
  use crate::{reset_test_env, test_helper::*, window::DelayEvent};

  #[test]
  fn smoke() {
    reset_test_env!();
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = fn_widget! {
      @MockBox {
        size: ZERO_SIZE,
        auto_focus: true,
        on_chars: move |event| c_receive.borrow_mut().push_str(&event.chars)
      }
    };
    let mut wnd = TestWindow::new(widget);

    let test_text_case = "Hello 世界！";
    wnd.draw_frame();
    #[allow(deprecated)]
    test_text_case.chars().for_each(|c| {
      if let Some(focus) = wnd.focusing() {
        wnd.add_delay_event(DelayEvent::Chars { id: focus, chars: c.into() });
      }
    });
    wnd.run_frame_tasks();

    assert_eq!(&*receive.borrow(), test_text_case);
  }

  #[test]
  fn chars_capture() {
    reset_test_env!();
    let receive = Rc::new(RefCell::new("".to_string()));
    let chars_receive = receive.clone();
    let capture_receive = receive.clone();

    let widget = fn_widget! {
      @MockBox {
        size: ZERO_SIZE,
        on_chars_capture: move |event| {
          let chars = event.chars.to_string();
          // The value received first is multiplied by 2
          let char = (chars.parse::<i32>().unwrap() * 2).to_string();
          capture_receive.borrow_mut().push_str(&char);
        },
        @MockBox {
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
    test_text_case.chars().for_each(|c| {
      if let Some(focus) = wnd.focusing() {
        wnd.add_delay_event(DelayEvent::Chars { id: focus, chars: c.into() });
      }
    });
    wnd.run_frame_tasks();
    assert_eq!(&*receive.borrow(), "214263");
  }
}
