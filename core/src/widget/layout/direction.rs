#[derive(Debug, Copy, Clone, PartialEq)]
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
