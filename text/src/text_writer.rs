use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::GraphemeCursor;
pub trait CharacterCursor {
  fn measure_bytes(&self, text: &str, byte_from: usize, char_len: usize) -> usize;

  fn move_by_char(&mut self, text: &str, offset: isize);

  fn set_to(&mut self, text: &str, pos: usize);

  fn next(&mut self, text: &str) -> bool;

  fn prev(&mut self, text: &str) -> bool;

  fn byte_offset(&self) -> usize;

  fn reset(&mut self, byte_offset: usize);
}

pub struct TextWriter<T = GraphemeCursor>
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

  pub fn text(&self) -> &str { &self.text }

  pub fn byte_offset(&self) -> usize { self.cursor.byte_offset() }

  pub fn set_to(&mut self, byte_offset: usize) {
    assert!(byte_offset <= self.text.len());
    self.cursor.reset(byte_offset);
  }

  pub fn move_by_char(&mut self, offset: isize) { self.cursor.move_by_char(&self.text, offset); }

  pub fn insert_chars(&mut self, s: &str) {
    self.text.insert_str(self.cursor.byte_offset(), s);
    self
      .cursor
      .reset(self.cursor.byte_offset() + s.len());
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
    self
      .text
      .insert_str(self.cursor.byte_offset(), text);
    self
      .cursor
      .reset(self.cursor.byte_offset() + text.len());
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

pub fn select_word(text: &str, cluster: usize) -> Range<usize> {
  let start = select_prev_word(text, cluster, true).start;
  let mut base = start;
  let it = text[start..].split_word_bounds();
  for word in it {
    if base + word.len() > cluster {
      return Range { start: base, end: base + word.len() };
    }
    base += word.len();
  }
  Range { start: text.len(), end: text.len() }
}

pub fn select_next_word(text: &str, cluster: usize, skip_whitespace: bool) -> Range<usize> {
  let it = text[cluster..].split_word_bound_indices();
  for (i, word) in it {
    if skip_whitespace && word.trim().is_empty() {
      continue;
    }
    return Range { start: cluster + i, end: cluster + i + word.len() };
  }
  Range { start: text.len(), end: text.len() }
}

pub fn select_prev_word(text: &str, cluster: usize, skip_whitespace: bool) -> Range<usize> {
  let mut it = text[..cluster].split_word_bound_indices();
  while let Some((i, word)) = it.next_back() {
    if skip_whitespace && word.trim().is_empty() {
      continue;
    }
    return Range { start: i, end: i + word.len() };
  }
  Range { start: 0, end: 0 }
}

#[cfg(test)]
mod tests {
  use crate::text_writer::select_prev_word;

  #[test]
  fn test_select_word() {
    use super::select_word;
    assert_eq!(select_word("hello,   my number is 123456", 0), 0..5); // hello
    assert_eq!(select_word("hello,   my number is 123456", 5), 5..6); // ,
    assert_eq!(select_word("hello,   my number is 123456", 7), 6..9); // "   "
    assert_eq!(select_word("hello,   my number is 123456", 9), 9..11); // my
    assert_eq!(select_word("hello,   my number is 123456", 12), 12..18); // number
    assert_eq!(select_word("hello,   my number is 123456", 19), 19..21); // is
    assert_eq!(select_word("hello,   my number is 123456", 22), 22..28); //123456
  }

  #[test]
  fn test_move_by_word() {
    use super::select_next_word;
    // hello
    assert_eq!(select_next_word("hello,   my number is 123456", 0, false), 0..5);
    // lo
    assert_eq!(select_next_word("hello,   my number is 123456", 3, false), 3..5);
    // ,
    assert_eq!(select_next_word("hello,   my number is 123456", 5, false), 5..6);

    // "   "
    assert_eq!(select_next_word("hello,   my number is 123456", 6, false), 6..9);

    // my
    assert_eq!(select_next_word("hello,   my number is 123456", 6, true), 9..11);

    // number
    assert_eq!(select_next_word("hello,   my number is 123456", 11, true), 12..18);

    // is
    assert_eq!(select_next_word("hello,   my number is 123456", 18, true), 19..21);

    //3456
    assert_eq!(select_next_word("hello,   my number is 123456", 24, false), 24..28);

    // 123456
    assert_eq!(select_prev_word("hello,   my number is 123456", 28, false), 22..28);

    // is
    assert_eq!(select_prev_word("hello,   my number is 123456", 21, false), 19..21);

    // numb
    assert_eq!(select_prev_word("hello,   my number is 123456", 16, false), 12..16);

    // " "
    assert_eq!(select_prev_word("hello,   my number is 123456", 12, false), 11..12);

    // my
    assert_eq!(select_prev_word("hello,   my number is 123456", 12, true), 9..11);

    // ,
    assert_eq!(select_prev_word("hello,   my number is 123456", 5, false), 0..5);

    // hel
    assert_eq!(select_prev_word("hello,   my number is 123456", 3, false), 0..3);
  }
}
