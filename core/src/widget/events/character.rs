use crate::prelude::*;
use rxrust::prelude::*;
use std::rc::Rc;

/// A widget that sends a single Unicode codepoint. The character can be pushed
/// to the end of a string.
pub type CharListener<W> = AttrWidget<W, CharAttr>;

#[derive(Debug, Default)]
pub struct CharAttr(LocalSubject<'static, Rc<CharEvent>, ()>);

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

impl<W> CharListener<W> {
  pub fn from_widget<A: AttachAttr<W = W>>(widget: A) -> Self {
    let (major, mut others, widget) = widget.take_attr::<CharAttr>();

    let major = major.unwrap_or_else(|| {
      let attrs = others.get_or_insert_with(<_>::default);
      if attrs.find_attr::<FocusAttr>().is_none() {
        attrs.front_push_attr(FocusAttr::default());
      }

      CharAttr::default()
    });

    CharListener { major, others, widget }
  }

  #[inline]
  pub fn event_observable(&self) -> LocalSubject<'static, Rc<CharEvent>, ()> {
    self.major.event_observable()
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

impl CharAttr {
  #[inline]
  pub fn event_observable(&self) -> LocalSubject<'static, Rc<CharEvent>, ()> { self.0.clone() }
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

    let widget = SizedBox::from_size(Size::zero())
      .with_auto_focus(true)
      .on_char(move |key| {
        c_receive.borrow_mut().push(key.char);
      });
    let mut wnd = window::NoRenderWindow::without_render(widget.box_it(), Size::new(100., 100.));

    let test_text_case = "Hello 世界！";
    wnd.render_ready();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
