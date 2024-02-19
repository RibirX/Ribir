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

pub mod flex;
mod sized_box;
pub use flex::*;
pub use sized_box::SizedBox;
pub mod expanded;
pub use expanded::Expanded;
mod stack;
pub use stack::*;
pub mod constrained_box;
pub use constrained_box::ConstrainedBox;
pub mod only_sized_by_parent;
pub use only_sized_by_parent::OnlySizedByParent;
pub use ribir_core::builtin_widgets::container::Container;
