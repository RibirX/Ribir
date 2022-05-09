use crate::prelude::*;
use rxrust::prelude::*;
use std::ptr::NonNull;

/// An attribute that sends a single Unicode codepoint. The character can be
/// pushed to the end of a string.
#[derive(Default)]
pub struct CharAttr(LocalSubject<'static, NonNull<CharEvent>, ()>);

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

impl std::borrow::Borrow<EventCommon> for CharEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for CharEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl CharAttr {
  #[inline]
  pub fn dispatch_event(&self, event: &mut CharEvent) { self.0.clone().next(NonNull::from(event)) }

  pub fn listen_on<H: FnMut(&mut CharEvent) + 'static>(
    &self,
    mut handler: H,
  ) -> SubscriptionWrapper<MutRc<SingleSubscription>> {
    self
      .0
      .clone()
      // Safety: Inner pointer from a mut reference and pass to handler one by one.
      .subscribe(move |mut event| handler(unsafe { event.as_mut() }))
  }
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

    let widget = SizedBox { size: SizedBox::shrink_size() }
      .with_auto_focus(true)
      .on_char(move |key| c_receive.borrow_mut().push(key.char));

    let mut wnd = Window::without_render(widget.box_it(), Size::new(100., 100.));

    let test_text_case = "Hello 世界！";
    wnd.render_ready();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
