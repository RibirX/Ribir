use std::ops::{Deref, DerefMut};

use ribir_core::prelude::{
  AppCtx, CharsEvent, GraphemeCursor, KeyCode, KeyboardEvent, NamedKey, PhysicalKey, StateWriter,
  TextWriter, VirtualKey,
};

use super::EditableText;

pub struct TextCaretWriter<'a, H>
where
  H: EditableText,
{
  host: &'a mut H,
  writer: TextWriter<GraphemeCursor>,
}

impl<'a, H> TextCaretWriter<'a, H>
where
  H: EditableText,
{
  pub fn new(host: &'a mut H) -> Self {
    let cursor = GraphemeCursor(host.caret().cluster());
    let string = host.text().to_string();
    Self { host, writer: TextWriter::new(string, cursor) }
  }
}

impl<'a, H> Drop for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  fn drop(&mut self) {
    use crate::input::caret_state::CaretPosition;
    let Self { host, writer } = self;
    let text = writer.text().to_string();
    let caret = CaretPosition { cluster: writer.byte_offset(), position: None };

    host.set_text_with_caret(&text, caret.into());
  }
}

impl<'a, H> Deref for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  type Target = TextWriter<GraphemeCursor>;
  fn deref(&self) -> &Self::Target { &self.writer }
}

impl<'a, H> DerefMut for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.writer }
}

pub(crate) fn edit_handle<F: EditableText>(this: &impl StateWriter<Value = F>, event: &CharsEvent) {
  if event.common.with_command_key() {
    return;
  }
  let chars = event
    .chars
    .chars()
    .filter(|c| !c.is_control() || c.is_ascii_whitespace())
    .collect::<String>();
  if !chars.is_empty() {
    let mut this = this.write();
    let rg = this.caret().select_range();
    let mut writer = TextCaretWriter::new(&mut *this);
    writer.delete_byte_range(&rg);
    writer.insert_str(&chars);
  }
}

pub(crate) fn edit_key_handle<F: EditableText>(
  this: &impl StateWriter<Value = F>, event: &KeyboardEvent,
) {
  let mut deal = false;
  if event.with_command_key() {
    deal = key_with_command(this, event)
  }
  if !deal {
    single_key(this, event);
  }
}

fn key_with_command<F: EditableText>(
  this: &impl StateWriter<Value = F>, event: &KeyboardEvent,
) -> bool {
  if !event.with_command_key() {
    return false;
  }

  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.key_code() {
    PhysicalKey::Code(KeyCode::KeyV) => {
      let clipboard = AppCtx::clipboard();
      let txt = clipboard.borrow_mut().read_text();
      if let Ok(txt) = txt {
        let mut this = this.write();
        let rg = this.caret().select_range();
        let mut writer = TextCaretWriter::new(&mut *this);
        if !rg.is_empty() {
          writer.delete_byte_range(&rg);
        }
        writer.insert_chars(&txt);
      }
      true
    }
    PhysicalKey::Code(KeyCode::KeyX) => {
      let rg = this.read().caret().select_range();
      if !rg.is_empty() {
        let mut this = this.write();
        let txt = this.text().substr(rg.clone()).to_string();
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&txt);
      }
      true
    }
    _ => false,
  }
}

fn single_key<F: EditableText>(this: &impl StateWriter<Value = F>, key: &KeyboardEvent) -> bool {
  match key.key() {
    VirtualKey::Named(NamedKey::Backspace) => {
      let mut this = this.write();
      let rg = this.caret().select_range();
      if rg.is_empty() {
        TextCaretWriter::new(&mut *this).back_space();
      } else {
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
      }
    }
    VirtualKey::Named(NamedKey::Delete) => {
      let mut this = this.write();
      let rg = this.caret().select_range();
      if rg.is_empty() {
        TextCaretWriter::new(&mut *this).del_char();
      } else {
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
      }
    }
    _ => (),
  };
  true
}
