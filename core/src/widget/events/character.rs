use crate::prelude::*;
use rxrust::prelude::*;
use std::rc::Rc;

/// A widget that sends a single Unicode codepoint. The character can be pushed
/// to the end of a string.
#[derive(Debug)]
pub struct CharListener {
  widget: BoxWidget,
  subject: LocalSubject<'static, Rc<CharEvent>, ()>,
}

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

widget::inherit_widget!(CharListener, widget);

impl CharListener {
  pub fn from_widget(widget: BoxWidget) -> BoxWidget {
    widget::inherit(
      FocusListener::from_widget(widget.box_it(), None, None),
      |base| Self {
        widget: base,
        subject: <_>::default(),
      },
      |_| {},
    )
  }

  #[inline]
  pub fn event_observable(&self) -> LocalSubject<'static, Rc<CharEvent>, ()> {
    self.subject.clone()
  }
}

impl std::convert::AsRef<EventCommon> for CharEvent {
  #[inline]
  fn as_ref(&self) -> &EventCommon { &self.common }
}

impl std::convert::AsMut<EventCommon> for CharEvent {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;
  use winit::event::WindowEvent;

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = SizedBox::empty_box(Size::zero())
      .with_auto_focus(true)
      .on_char(move |key| {
        c_receive.borrow_mut().push(key.char);
      });
    let mut wnd = window::NoRenderWindow::without_render(widget, Size::new(100., 100.));

    let test_text_case = "Hello 世界！";
    wnd.render_ready();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
