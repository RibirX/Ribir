#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
  #[default]
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

mod column;
pub mod container;
pub mod flex;
mod row;
mod sized_box;
pub use column::Column;
pub use flex::*;
pub use row::Row;
pub use sized_box::SizedBox;
pub mod expanded;
pub use container::Container;
pub use expanded::Expanded;
mod stack;
pub use stack::*;
mod expand_box;
pub use expand_box::*;
