use ribir_core::prelude::{
  CharacterCursor, ControlChar, KeyboardEvent, TextWriter, VirtualKeyCode,
};

use super::Input;

impl Input {
  pub(crate) fn edit_handle(&mut self, c: char) {
    let has_deleted = self.del_selected();

    let mut cursor = self.caret.cursor();
    let mut writer = TextWriter::new(&mut self.text, &mut cursor);
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
    self.caret = cursor.byte_offset().into();
  }

  pub(crate) fn key_handle(&mut self, key: &mut KeyboardEvent) {
    let mut cursor = self.caret.cursor();
    let mut writer = TextWriter::new(&mut self.text, &mut cursor);
    match key.key {
      VirtualKeyCode::Left => {
        writer.move_to_prev();
        self.caret = cursor.byte_offset().into();
      }
      VirtualKeyCode::Right => {
        writer.move_to_next();
        self.caret = cursor.byte_offset().into();
      }
      _ => (),
    };
  }

  pub(crate) fn del_selected(&mut self) -> bool {
    let (begin, end) = self.caret.select_range();
    if begin == end {
      return false;
    }
    self.text.drain(begin..end);
    self.caret = begin.into();
    return true;
  }
}
