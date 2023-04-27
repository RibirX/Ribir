#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
  #[default]
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
  Top,
  Bottom,
  Left,
  Right,
}

impl Direction {
  pub fn is_horizontal(&self) -> bool { matches!(self, Direction::Horizontal) }

  pub fn is_vertical(&self) -> bool { matches!(self, Direction::Vertical) }
}

mod column;
pub mod flex;
mod row;
mod sized_box;
pub use column::Column;
pub use flex::*;
pub use row::Row;
pub use sized_box::SizedBox;
pub mod expanded;
pub use expanded::Expanded;
pub mod constrained_box;
pub use constrained_box::ConstrainedBox;
pub use ribir_core::builtin_widgets::container::*;
pub use ribir_core::builtin_widgets::stack::*;
