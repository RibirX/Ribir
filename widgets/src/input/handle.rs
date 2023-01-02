use std::ops::{Deref, DerefMut, Range};

use ribir_core::prelude::{ControlChar, GraphemeCursor, KeyboardEvent, TextWriter, VirtualKeyCode};

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
  pub(crate) fn edit_handle(&mut self, c: char) {
    let has_deleted = self.del_selected();

    let mut writer = InputWriter::new(self);
    match c {
      ControlChar::DEL => {
        if !has_deleted {
          writer.del_char()
        }
      }
      ControlChar::BACKSPACE => {
        if !has_deleted {
          writer.back_space()
        }
      }
      _ => writer.insert_char(c),
    };
  }

  pub(crate) fn key_handle(&mut self, key: &mut KeyboardEvent) {
    match key.key {
      VirtualKeyCode::Left => {
        InputWriter::new(self).move_to_prev();
      }
      VirtualKeyCode::Right => {
        InputWriter::new(self).move_to_next();
      }
      _ => (),
    };
  }

  pub(crate) fn del_selected(&mut self) -> bool {
    let (start, end) = self.caret.select_range();
    if start == end {
      return false;
    }
    InputWriter::new(self).delete_byte_range(&Range { start, end });
    true
  }
}
