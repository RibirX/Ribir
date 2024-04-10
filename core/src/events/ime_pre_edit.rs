use crate::{impl_common_event_deref, prelude::*};

#[derive(Debug)]
pub enum ImePreEdit {
  /// Notifies when the IME PreEdit begin a new round.
  ///
  /// After getting this event you could receive [`PreEdit`](Self::PreEdit).
  Begin,

  /// Notifies when a new composing text should be set at the cursor position.
  ///
  /// The value represents a pair of the preedit string and the cursor begin
  /// position and end position. When it's `None`, the cursor should be
  /// hidden. When `String` is an empty string this indicates that preedit was
  /// cleared.
  ///
  /// The cursor position is byte-wise indexed.
  PreEdit { value: String, cursor: Option<(usize, usize)> },

  /// Notifies when the IME PreEdit was finished this round.
  ///
  /// After receiving this event you won't get any more PreEdit event in this
  /// round.You should clear pending pre_edit text.
  End,
}

#[derive(Debug)]
pub struct ImePreEditEvent {
  pub pre_edit: ImePreEdit,
  pub common: CommonEvent,
}

impl ImePreEditEvent {
  pub(crate) fn new(pre_edit: ImePreEdit, target: WidgetId, wnd: &Window) -> Self {
    ImePreEditEvent { pre_edit, common: CommonEvent::new(target, wnd.id()) }
  }
}

impl_common_event_deref!(ImePreEditEvent);
