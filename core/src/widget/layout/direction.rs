// todo: support the direction in horizontal and vertical
use crate::prelude::StatePartialEq;

#[derive(Debug, Copy, Clone, PartialEq, StatePartialEq)]
pub enum Direction {
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

impl Default for Direction {
  #[inline]
  fn default() -> Self { Direction::Horizontal }
}
