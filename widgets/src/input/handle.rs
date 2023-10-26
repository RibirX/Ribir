use std::ops::{Deref, DerefMut};

use ribir_core::prelude::{
  AppCtx, CharsEvent, GraphemeCursor, KeyCode, KeyboardEvent, NamedKey, PhysicalKey, TextWriter,
  VirtualKey,
};

#[macro_export]
macro_rules! declare_writer {
  ($writer: ident, $host: ident) => {
    struct $writer<'a> {
      input: &'a mut $host,
      writer: TextWriter<GraphemeCursor>,
    }

    impl<'a> $writer<'a> {
      fn new(input: &'a mut $host) -> Self {
        let cursor = GraphemeCursor(input.caret.cluster());
        let string = input.text.to_string();
        Self {
          input,
          writer: TextWriter::new(string, cursor),
        }
      }
    }

    impl<'a> Drop for $writer<'a> {
      fn drop(&mut self) {
        use $crate::input::caret_state::CaretPosition;
        let Self { input, writer } = self;
        input.caret = CaretPosition {
          cluster: writer.byte_offset(),
          position: None,
        }
        .into();
        input.text = writer.text().clone().into();
      }
    }

    impl<'a> Deref for $writer<'a> {
      type Target = TextWriter<GraphemeCursor>;
      fn deref(&self) -> &Self::Target { &self.writer }
    }

    impl<'a> DerefMut for $writer<'a> {
      fn deref_mut(&mut self) -> &mut Self::Target { &mut self.writer }
    }
  };
}

declare_writer!(InputWriter, TextEditorArea);
use super::TextEditorArea;
impl TextEditorArea {
  pub(crate) fn edit_handle(this: &mut TextEditorArea, event: &CharsEvent) {
    if event.common.with_command_key() {
      return;
    }
    let chars = event
      .chars
      .chars()
      .filter(|c| !c.is_control())
      .collect::<String>();
    if !chars.is_empty() {
      let rg = this.caret.select_range();
      let mut writer = InputWriter::new(this);
      writer.delete_byte_range(&rg);
      writer.insert_str(&chars);
    }
  }

  pub(crate) fn key_handle(this: &mut TextEditorArea, event: &KeyboardEvent) {
    let mut deal = false;
    if event.with_command_key() {
      deal = key_with_command(this, event)
    }
    if !deal {
      single_key(this, event);
    }
  }
}

fn key_with_command(this: &mut TextEditorArea, event: &KeyboardEvent) -> bool {
  if !event.with_command_key() {
    return false;
  }

  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.physical_key {
    PhysicalKey::Code(KeyCode::KeyV) => {
      let clipboard = AppCtx::clipboard();
      let txt = clipboard.borrow_mut().read_text();
      if let Ok(txt) = txt {
        let rg = this.caret.select_range();
        let mut writer = InputWriter::new(this);
        if !rg.is_empty() {
          writer.delete_byte_range(&rg);
        }
        writer.insert_chars(&txt);
      }
      true
    }
    PhysicalKey::Code(KeyCode::KeyX) => {
      let rg = this.caret.select_range();
      if !rg.is_empty() {
        let txt = this.text.substr(rg.clone()).to_string();
        InputWriter::new(this).delete_byte_range(&rg);
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&txt);
      }
      true
    }
    _ => false,
  }
}

fn single_key(this: &mut TextEditorArea, key: &KeyboardEvent) -> bool {
  match key.key {
    VirtualKey::Named(NamedKey::Enter) => {
      if this.multi_line {
        InputWriter::new(this).insert_str("\r");
      }
    }
    VirtualKey::Named(NamedKey::Backspace) => {
      let rg = this.caret.select_range();
      if rg.is_empty() {
        InputWriter::new(this).back_space();
      } else {
        InputWriter::new(this).delete_byte_range(&rg);
      }
    }
    VirtualKey::Named(NamedKey::Delete) => {
      let rg = this.caret.select_range();
      if rg.is_empty() {
        InputWriter::new(this).del_char();
      } else {
        InputWriter::new(this).delete_byte_range(&rg);
      }
    }
    _ => (),
  };
  true
}
