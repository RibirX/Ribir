use crate::{impl_common_event_deref, prelude::*};
use crate::{impl_compose_child_for_listener, impl_listener};
use std::convert::Infallible;

#[derive(Debug)]
pub enum ImePreEdit {
  /// Notifies when the IME was enabled.
  ///
  /// After getting this event you could receive [`Preedit`](Self::Preedit).
  /// You should also start performing IME related requests like
  /// [`Window::set_ime_cursor_area`].
  Begin,

  /// Notifies when a new composing text should be set at the cursor position.
  ///
  /// The value represents a pair of the preedit string and the cursor begin
  /// position and end position. When it's `None`, the cursor should be
  /// hidden. When `String` is an empty string this indicates that preedit was
  /// cleared.
  ///
  /// The cursor position is byte-wise indexed.
  PreEdit {
    value: String,
    cursor: Option<(usize, usize)>,
  },

  /// Notifies when the IME was disabled.
  ///
  /// After receiving this event you won't get any more PreEdit event in this
  /// round.You should also stop issuing IME related requests like
  /// [`Window::set_ime_cursor_area`] and clear pending preedit text.
  End,
}

#[derive(Debug)]
pub struct ImePreEditEvent {
  pub pre_edit: ImePreEdit,
  pub common: CommonEvent,
}

impl ImePreEditEvent {
  pub(crate) fn new(pre_edit: ImePreEdit, target: WidgetId, wnd: &Window) -> Self {
    ImePreEditEvent {
      pre_edit,
      common: CommonEvent::new(target, wnd.id()),
    }
  }
}

pub type ImePreEditSubject = MutRefItemSubject<'static, ImePreEditEvent, Infallible>;

impl_listener! {
  "The listener use to listen ime pre edit events.",
  ImePreEdit,
  ImePreEditEvent
}

impl_common_event_deref!(ImePreEditEvent);

impl_compose_child_for_listener!(ImePreEditListener);

impl ImePreEditListener {
  pub fn on_ime_pre_edit(mut self, handler: impl FnMut(&mut ImePreEditEvent) + 'static) -> Self {
    self.subject().subscribe(handler);
    self
  }
}
