use unicode_segmentation::GraphemeCursor as _GraphemeCursor;

use crate::CharacterCursor;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct GraphemeCursor(pub usize);

impl CharacterCursor for GraphemeCursor {
  fn measure_bytes(&self, text: &str, byte_from: usize, mut char_len: usize) -> usize {
    let mut legacy = _GraphemeCursor::new(byte_from, text.len(), true);
    while char_len > 0 {
      char_len -= 1;
      if legacy.next_boundary(text, 0).unwrap().is_none() {
        break;
      }
    }
    legacy.cur_cursor() - byte_from
  }

  fn move_by_char(&mut self, text: &str, mut offset: isize) {
    let next = offset > 0;
    offset = offset.abs();
    while offset > 0 {
      if !match next {
        true => self.next(text),
        false => self.prev(text),
      } {
        return;
      }
      offset -= 1;
    }
  }

  fn next(&mut self, text: &str) -> bool {
    if self.0 == text.len() {
      return false;
    }

    self.0 += self.measure_bytes(text, self.0, 1);
    true
  }

  fn prev(&mut self, text: &str) -> bool {
    let mut legacy = _GraphemeCursor::new(self.0, text.len(), true);
    if let Some(len) = legacy.prev_boundary(text, 0).unwrap() {
      self.0 = len;
      true
    } else {
      false
    }
  }

  fn set_to(&mut self, text: &str, char_pos: usize) {
    self.0 = self.measure_bytes(text, 0, char_pos);
  }

  fn byte_offset(&self) -> usize { self.0 }

  fn reset(&mut self, byte_offset: usize) { self.0 = byte_offset; }
}

#[cfg(test)]
mod tests {

  use super::*;
  #[test]
  fn test_compose_emoj() {
    let text = "👨‍👩‍👦‍👦";
    let mut cursor = GraphemeCursor(0);
    assert!(25 == cursor.measure_bytes(text, 0, 1));
    cursor.next(text);
    assert!(25 == cursor.byte_offset());
    cursor.prev(text);
    assert!(0 == cursor.byte_offset());
  }
  #[test]
  fn test_char_with_combine() {
    let text = "ee\u{0301}e\u{0301}\u{0301}";
    let mut cursor = GraphemeCursor(0);
    cursor.next(text);
    assert!(1 == cursor.byte_offset());
    cursor.next(text);
    assert!(4 == cursor.byte_offset());
    cursor.next(text);
    assert!(9 == cursor.byte_offset());

    cursor.prev(text);
    assert!(4 == cursor.byte_offset());
    cursor.prev(text);
    assert!(1 == cursor.byte_offset());
    cursor.prev(text);
    assert!(0 == cursor.byte_offset());
  }
}
