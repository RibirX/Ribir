#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
  #[default]
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}
