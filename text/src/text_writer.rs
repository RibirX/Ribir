use std::ops::Range;

pub struct ControlChar;

#[allow(dead_code)]
impl ControlChar {
  pub const BACKSPACE: char = '\u{8}';
  pub const DEL: char = '\u{7f}';
}

pub trait CharacterCursor {
  fn measure_bytes(&self, text: &str, byte_from: usize, char_len: usize) -> usize;

  fn move_by_char(&mut self, text: &str, offset: isize);

  fn set_to(&mut self, text: &str, pos: usize);

  fn next(&mut self, text: &str) -> bool;

  fn prev(&mut self, text: &str) -> bool;

  fn byte_offset(&self) -> usize;

  fn reset(&mut self, byte_offset: usize);
}

pub struct TextWriter<T>
where
  T: CharacterCursor,
{
  text: String,
  cursor: T,
}

impl<T> TextWriter<T>
where
  T: CharacterCursor,
{
  pub fn new(text: String, cursor: T) -> Self { Self { text, cursor } }

  pub fn dispose(self) -> (String, T) { (self.text, self.cursor) }

  pub fn text(&self) -> &String { &self.text }

  pub fn byte_offset(&self) -> usize { self.cursor.byte_offset() }

  pub fn insert_char(&mut self, c: char) {
    self.text.insert(self.cursor.byte_offset(), c);
    self.cursor.next(&self.text);
  }

  pub fn del_char(&mut self) {
    if self.is_at_last() {
      return;
    }
    let idx = self.cursor.byte_offset();
    let len = self.cursor.measure_bytes(&self.text, idx, 1);

    self.delete_byte_range(&Range { start: idx, end: idx + len })
  }

  pub fn back_space(&mut self) {
    if self.move_to_prev() {
      self.del_char();
    }
  }

  pub fn move_to_next(&mut self) -> bool {
    if self.is_at_last() {
      return false;
    }

    self.cursor.next(&self.text)
  }

  pub fn move_to_prev(&mut self) -> bool {
    if self.cursor.byte_offset() == 0 {
      return false;
    }
    self.cursor.prev(&self.text)
  }

  pub fn is_at_last(&self) -> bool { self.text.len() <= self.cursor.byte_offset() }

  pub fn insert_str(&mut self, text: &str) {
    self.text.insert_str(self.cursor.byte_offset(), text);
    self.cursor.reset(self.cursor.byte_offset() + text.len());
  }

  pub fn delete_byte_range(&mut self, rg: &Range<usize>) {
    self.text.drain(rg.clone());

    let cursor = self.cursor.byte_offset();
    let new_cursor = match cursor {
      _ if rg.contains(&cursor) => rg.start,
      _ if (rg.end <= cursor) => cursor - (rg.end - rg.start),
      _ => cursor,
    };
    self.cursor.reset(new_cursor);
  }
}
