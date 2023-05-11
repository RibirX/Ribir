use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretState {
  Caret(usize),
  Select(usize, usize),
  Selecting(usize, usize),
}

impl From<usize> for CaretState {
  fn from(c: usize) -> Self { CaretState::Caret(c) }
}

impl From<u32> for CaretState {
  fn from(c: u32) -> Self { CaretState::Caret(c as usize) }
}

impl From<(usize, usize)> for CaretState {
  fn from((begin, end): (usize, usize)) -> Self { CaretState::Select(begin, end) }
}

impl Default for CaretState {
  fn default() -> Self { 0_u32.into() }
}

impl CaretState {
  pub fn select_range(&self) -> Range<usize> {
    match *self {
      CaretState::Caret(cursor) => Range { start: cursor, end: cursor },
      CaretState::Select(begin, end) => Range {
        start: begin.min(end),
        end: begin.max(end),
      },
      CaretState::Selecting(begin, end) => Range {
        start: begin.min(end),
        end: begin.max(end),
      },
    }
  }

  pub fn offset(&self) -> usize {
    match *self {
      CaretState::Caret(cursor) => cursor,
      CaretState::Select(_, end) => end,
      CaretState::Selecting(_, end) => end,
    }
  }

  pub fn valid(&mut self, len: usize) {
    *self = match *self {
      CaretState::Caret(cursor) => CaretState::Caret(cursor.min(len)),
      CaretState::Select(begin, end) => {
        let begin = begin.min(len);
        let end = end.min(len);
        if begin == end {
          CaretState::Caret(begin)
        } else {
          CaretState::Select(begin, end)
        }
      }
      CaretState::Selecting(begin, end) => CaretState::Selecting(begin.min(len), end.min(len)),
    };
  }
}
