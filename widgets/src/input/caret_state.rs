use ribir_core::prelude::GraphemeCursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretState {
  Caret(usize),
  Select(usize, usize),
  Selecting(usize, usize),
}

impl From<usize> for CaretState {
  fn from(c: usize) -> Self { CaretState::Caret(c) }
}

impl From<(usize, usize)> for CaretState {
  fn from((begin, end): (usize, usize)) -> Self { CaretState::Select(begin, end) }
}

impl Default for CaretState {
  fn default() -> Self { 0.into() }
}

impl CaretState {
  pub fn select_range(&self) -> (usize, usize) {
    match *self {
      CaretState::Caret(cursor) => (cursor, cursor),
      CaretState::Select(begin, end) => (begin.min(end), begin.max(end)),
      CaretState::Selecting(begin, end) => (begin.min(end), begin.max(end)),
    }
  }

  pub fn cursor(&self) -> GraphemeCursor { GraphemeCursor(self.offset()) }

  pub fn offset(&self) -> usize {
    match *self {
      CaretState::Caret(cursor) => cursor,
      CaretState::Select(_, end) => end,
      CaretState::Selecting(_, end) => end,
    }
  }
}
