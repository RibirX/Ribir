use crate::{
  impl_query_self_only,
  prelude::{data_widget::compose_child_as_data_widget, *},
};

/// An attribute that sends a single Unicode codepoint. The character can be
/// pushed to the end of a string.
#[derive(Declare)]
pub struct CharListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(&'r mut CharEvent)))]
  on_char: Box<dyn for<'r> FnMut(&'r mut CharEvent)>,
}

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

impl ComposeSingleChild for CharListener {
  #[inline]
  fn compose_single_child(
    this: StateWidget<Self>,
    child: Option<Widget>,
    _: &mut BuildCtx,
  ) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for CharListener {
  impl_query_self_only!();
}

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

impl EventListener for CharListener {
  type Event = CharEvent;
  #[inline]
  fn dispatch(&mut self, event: &mut CharEvent) { (self.on_char)(event) }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = widget! {
      SizedBox {
        size: ZERO_SIZE,
        auto_focus: true,
        on_char: move |key| c_receive.borrow_mut().push(key.char)
      }
    };
    let mut wnd = Window::without_render(widget.into_widget(), Size::new(100., 100.));

    let test_text_case = "Hello 世界！";
    wnd.draw_frame();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
