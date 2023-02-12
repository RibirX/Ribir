use std::ops::{Deref, DerefMut};

use ribir_core::prelude::{CharEvent, GraphemeCursor, KeyboardEvent, TextWriter, VirtualKeyCode};

struct InputWriter<'a> {
  input: &'a mut Input,
  writer: TextWriter<GraphemeCursor>,
}

impl<'a> InputWriter<'a> {
  fn new(input: &'a mut Input) -> Self {
    let cursor = GraphemeCursor(input.caret.offset());
    let string = input.text().to_string();
    Self {
      input,
      writer: TextWriter::new(string, cursor),
    }
  }
}

impl<'a> Drop for InputWriter<'a> {
  fn drop(&mut self) {
    let Self { input, writer } = self;
    input.caret = writer.byte_offset().into();
    input.text = writer.text().clone().into();
  }
}

impl<'a> Deref for InputWriter<'a> {
  type Target = TextWriter<GraphemeCursor>;
  fn deref(&self) -> &Self::Target { &self.writer }
}

impl<'a> DerefMut for InputWriter<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.writer }
}

use super::Input;
impl Input {
  pub(crate) fn edit_handle(&mut self, event: &mut CharEvent) {
    if !event.char.is_ascii_control() {
      let rg = self.caret.select_range();
      let mut writer = InputWriter::new(self);
      writer.delete_byte_range(&rg);
      writer.insert_char(event.char);
    }
  }

  pub(crate) fn key_handle(&mut self, key: &mut KeyboardEvent) {
    match key.key {
      VirtualKeyCode::Left => {
        InputWriter::new(self).move_to_prev();
      }
      VirtualKeyCode::Right => {
        InputWriter::new(self).move_to_next();
      }
      VirtualKeyCode::Back => {
        let rg = self.caret.select_range();
        if rg.is_empty() {
          InputWriter::new(self).back_space();
        } else {
          InputWriter::new(self).delete_byte_range(&rg);
        }
      }
      VirtualKeyCode::Delete => {
        let rg = self.caret.select_range();
        if rg.is_empty() {
          InputWriter::new(self).del_char();
        } else {
          InputWriter::new(self).delete_byte_range(&rg);
        }
      }
      _ => (),
    };
  }
}
