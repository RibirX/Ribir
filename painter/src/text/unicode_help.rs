use crate::CharacterCursor;

// UTF-8 ranges and tags for encoding characters
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO_B: u8 = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8 = 0b1111_0000;

#[inline]
pub fn next_char_len(text: &str, byte_idx: usize) -> usize {
  let b: u8 = text.as_bytes()[byte_idx];
  if b == 0 {
    0
  } else if b < TAG_TWO_B {
    1
  } else if b < TAG_THREE_B {
    2
  } else if b < TAG_FOUR_B {
    3
  } else {
    4
  }
}

#[inline]
pub fn prev_char_len(text: &str, byte_idx: usize) -> usize {
  let bytes = text.as_bytes();
  let mut len = 0;
  while byte_idx > len {
    len += 1;
    let c = bytes[byte_idx - len];
    if !(TAG_CONT..TAG_TWO_B).contains(&c) {
      return len;
    }
  }
  len
}

#[inline]
pub fn measure_bytes(text: &str, byte_from: usize, mut char_len: usize) -> usize {
  let mut len = 0;
  while 0 < char_len {
    let c_len = next_char_len(text, byte_from);
    if c_len == 0 {
      break;
    }
    len += c_len;

    char_len -= 1;
  }

  len
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct UnicodeCursor(pub usize);

impl CharacterCursor for UnicodeCursor {
  fn measure_bytes(&self, text: &str, byte_from: usize, char_len: usize) -> usize {
    measure_bytes(text, byte_from, char_len)
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
    let len = next_char_len(text, self.byte_offset());
    if len > 0 {
      self.0 += len;
      true
    } else {
      false
    }
  }

  fn prev(&mut self, text: &str) -> bool {
    let len = prev_char_len(text, self.byte_offset());
    if len > 0 {
      self.0 -= len;
      true
    } else {
      false
    }
  }

  fn set_to(&mut self, text: &str, char_pos: usize) {
    let mut byte_idx = 0;
    let mut char_idx = 0;
    while char_idx < char_pos {
      let len = next_char_len(text, byte_idx);
      if len == 0 {
        break;
      }
      byte_idx += len;
      char_idx += 1;
    }
    self.0 = byte_idx;
  }

  fn byte_offset(&self) -> usize { self.0 }

  fn reset(&mut self, byte_offset: usize) { self.0 = byte_offset; }
}
