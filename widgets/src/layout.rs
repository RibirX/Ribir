#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
  #[default]
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

impl Direction {
  pub fn is_horizontal(&self) -> bool { matches!(self, Direction::Horizontal) }

  pub fn is_vertical(&self) -> bool { matches!(self, Direction::Vertical) }
}

pub mod text_clamp;
pub use text_clamp::*;
pub mod flex;
mod sized_box;
pub use flex::*;
pub use sized_box::*;
pub mod expanded;
pub use expanded::*;
mod stack;
pub use stack::*;
pub mod no_affected_parent_size;
pub use no_affected_parent_size::*;
mod fractionally;
pub use fractionally::*;
mod line;
pub use line::*;
