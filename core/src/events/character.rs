use crate::{impl_common_event_deref, prelude::*};

#[derive(Debug)]
pub struct CharsEvent {
  pub chars: CowArc<str>,
  pub common: CommonEvent,
}

impl_common_event_deref!(CharsEvent);

impl CharsEvent {
  #[inline]
  pub fn new(chars: CowArc<str>, id: WidgetId, wnd: &Window) -> Self {
    Self { chars, common: CommonEvent::new(id, wnd.tree) }
  }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{reset_test_env, test_helper::*, window::DelayEvent};

  #[test]
  fn smoke() {
    reset_test_env!();
    let (watcher, writer) = split_value("".to_string());

    let widget = fn_widget! {
      let writer = writer.clone_writer();
      @MockBox {
        size: ZERO_SIZE,
        auto_focus: true,
        on_chars: move |event| writer.write().push_str(&event.chars)
      }
    };

    let wnd = TestWindow::from_widget(widget);

    let test_text_case = "Hello 世界！";
    wnd.draw_frame();
    #[allow(deprecated)]
    test_text_case.chars().for_each(|c| {
      if let Some(focus) = wnd.focusing() {
        wnd.add_delay_event(DelayEvent::Chars { id: focus, chars: c.into() });
      }
    });
    wnd.run_frame_tasks();

    assert_eq!(*watcher.read(), test_text_case);
  }

  #[test]
  fn chars_capture() {
    reset_test_env!();

    let (reader, writer) = split_value("".to_string());

    let widget = fn_widget! {
      @MockBox {
        size: ZERO_SIZE,
        on_chars_capture: move |event| {
          let chars = event.chars.to_string();
          // The value received first is multiplied by 2
          let char = (chars.parse::<i32>().unwrap() * 2).to_string();
          $writer.write().push_str(&char);
        },
        @MockBox {
          size: ZERO_SIZE,
          auto_focus: true,
          on_chars: move |event| $writer.write().push_str(&event.chars),
        }
      }
    };
    let wnd = TestWindow::from_widget(widget);

    let test_text_case = "123";
    wnd.draw_frame();
    #[allow(deprecated)]
    test_text_case.chars().for_each(|c| {
      if let Some(focus) = wnd.focusing() {
        wnd.add_delay_event(DelayEvent::Chars { id: focus, chars: c.into() });
      }
    });
    wnd.run_frame_tasks();
    assert_eq!(&*reader.read(), "214263");
  }
}
