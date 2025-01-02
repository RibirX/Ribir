use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretState {
  Caret(CaretPosition),
  Select(CaretPosition, CaretPosition),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretPosition {
  pub cluster: usize,
  pub position: Option<(usize, usize)>,
}

impl Default for CaretState {
  fn default() -> Self { CaretState::Caret(CaretPosition { cluster: 0, position: None }) }
}

impl From<CaretPosition> for CaretState {
  fn from(position: CaretPosition) -> Self { CaretState::Caret(position) }
}

impl CaretState {
  pub fn select_range(&self) -> Range<usize> {
    match *self {
      CaretState::Caret(cursor) => Range { start: cursor.cluster, end: cursor.cluster },
      CaretState::Select(begin, end) => {
        Range { start: begin.cluster.min(end.cluster), end: begin.cluster.max(end.cluster) }
      }
    }
  }

  pub fn cluster(&self) -> usize {
    match *self {
      CaretState::Caret(cursor) | CaretState::Select(_, cursor) => cursor.cluster,
    }
  }

  pub fn caret_position(&self) -> CaretPosition {
    match *self {
      CaretState::Caret(cursor) | CaretState::Select(_, cursor) => cursor,
    }
  }
}
