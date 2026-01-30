use ribir_core::prelude::*;
pub mod expanded;
pub mod flex;
pub use expanded::*;
mod stack;
pub use flex::*;
pub use stack::*;
mod row_column;
pub use row_column::*;

/// The direction of a linear layout.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
  #[default]
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

/// How the children should be placed along the main axis in a linear layout.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum JustifyContent {
  /// Place the children as close to the start of the main axis as possible.
  #[default]
  Start,
  /// Place the children as close to the middle of the main axis as possible.
  Center,
  /// Place the children as close to the end of the main axis as possible.
  End,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items is the
  /// same. The first item is flush with the main-start edge, and the last
  /// item is flush with the main-end edge.
  SpaceBetween,
  /// The children are evenly distributed within the alignment container
  /// along the main axis. The spacing between each pair of adjacent items is
  /// the same. The empty space before the first and after the last item
  /// equals half of the space between each pair of adjacent items.
  SpaceAround,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items, the
  /// main-start edge and the first item, and the main-end edge and the last
  /// item, are all exactly the same.
  SpaceEvenly,
}

impl Direction {
  /// Returns `true` if the direction is horizontal.
  pub const fn is_horizontal(&self) -> bool { matches!(self, Direction::Horizontal) }

  /// Returns `true` if the direction is vertical.
  pub const fn is_vertical(&self) -> bool { matches!(self, Direction::Vertical) }

  pub const fn main_cross_of(self, size: Size) -> (f32, f32) {
    match self {
      Direction::Horizontal => (size.width, size.height),
      Direction::Vertical => (size.height, size.width),
    }
  }

  pub const fn to_point(self, main: f32, cross: f32) -> Point {
    match self {
      Direction::Horizontal => Point::new(main, cross),
      Direction::Vertical => Point::new(cross, main),
    }
  }

  pub const fn pos_of_main(self, pos: Point) -> f32 {
    match self {
      Direction::Horizontal => pos.x,
      Direction::Vertical => pos.y,
    }
  }

  pub const fn pos_of_cross(self, pos: Point) -> f32 {
    match self {
      Direction::Horizontal => pos.y,
      Direction::Vertical => pos.x,
    }
  }

  pub const fn to_size(self, main: f32, cross: f32) -> Size {
    match self {
      Direction::Horizontal => Size::new(main, cross),
      Direction::Vertical => Size::new(cross, main),
    }
  }
  pub const fn max_of(self, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => clamp.max.width,
      Direction::Vertical => clamp.max.height,
    }
  }

  pub const fn min_of(self, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => clamp.min.width,
      Direction::Vertical => clamp.min.height,
    }
  }

  pub const fn cross_max_of(self, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => clamp.max.height,
      Direction::Vertical => clamp.max.width,
    }
  }

  pub const fn cross_min_of(self, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => clamp.min.height,
      Direction::Vertical => clamp.min.width,
    }
  }

  /// Clamps the given value to the given clamp in the cross axis.
  pub const fn cross_clamp(self, value: f32, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => f32::clamp(value, clamp.min.height, clamp.max.height),
      Direction::Vertical => f32::clamp(value, clamp.min.width, clamp.max.width),
    }
  }

  /// Clamps the given value to the given clamp in the main axis.
  pub const fn main_clamp(self, value: f32, clamp: &BoxClamp) -> f32 {
    match self {
      Direction::Horizontal => f32::clamp(value, clamp.min.width, clamp.max.width),
      Direction::Vertical => f32::clamp(value, clamp.min.height, clamp.max.height),
    }
  }

  /// Returns the main axis size of the given size.
  pub const fn main_of(self, size: Size) -> f32 {
    match self {
      Direction::Horizontal => size.width,
      Direction::Vertical => size.height,
    }
  }

  /// Returns the cross axis size of the given size.
  pub const fn cross_of(self, size: Size) -> f32 {
    match self {
      Direction::Horizontal => size.height,
      Direction::Vertical => size.width,
    }
  }

  /// Creates a new `BoxClamp` with the given max value for the main axis.
  pub const fn with_max(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_max_width(value),
      Direction::Vertical => clamp.with_max_height(value),
    }
  }

  /// Creates a new `BoxClamp` with the given min value for the main axis.
  pub const fn with_min(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_min_width(value),
      Direction::Vertical => clamp.with_min_height(value),
    }
  }

  /// Creates a new `BoxClamp` with the given fixed value for the main axis.
  pub const fn with_fixed_main(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_fixed_width(value),
      Direction::Vertical => clamp.with_fixed_height(value),
    }
  }

  /// Creates a new `BoxClamp` with the given fixed value for the cross axis.
  pub const fn with_fixed_cross(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_fixed_height(value),
      Direction::Vertical => clamp.with_fixed_width(value),
    }
  }

  /// Creates a new `BoxClamp` with the given max value for the cross axis.
  pub const fn with_cross_max(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_max_height(value),
      Direction::Vertical => clamp.with_max_width(value),
    }
  }

  /// Creates a new `BoxClamp` with the given min value for the cross axis.
  pub const fn with_cross_min(self, clamp: BoxClamp, value: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_min_height(value),
      Direction::Vertical => clamp.with_min_width(value),
    }
  }

  /// Creates a new `BoxClamp` with the given min and max values for the main
  /// axis.
  pub const fn with_min_max(self, clamp: BoxClamp, min: f32, max: f32) -> BoxClamp {
    match self {
      Direction::Horizontal => clamp.with_min_width(min).with_max_width(max),
      Direction::Vertical => clamp.with_min_height(min).with_max_height(max),
    }
  }

  pub const fn container_main(self, clamp: &BoxClamp, child_main: f32) -> f32 {
    match self {
      Direction::Horizontal => clamp.container_width(child_main),
      Direction::Vertical => clamp.container_height(child_main),
    }
  }

  pub const fn container_cross(self, clamp: &BoxClamp, child_cross: f32) -> f32 {
    match self {
      Direction::Horizontal => clamp.container_height(child_cross),
      Direction::Vertical => clamp.container_width(child_cross),
    }
  }
}

impl JustifyContent {
  /// Returns the items' starting offset and the step between each item.
  pub(crate) fn item_offset_and_step(self, main_space_leave: f32, item_cnt: usize) -> (f32, f32) {
    if item_cnt == 0 || main_space_leave <= 0.0 {
      return (0.0, 0.0);
    }

    match self {
      JustifyContent::Start => (0.0, 0.0),
      JustifyContent::Center => (main_space_leave / 2.0, 0.0),
      JustifyContent::End => (main_space_leave, 0.0),
      JustifyContent::SpaceAround => {
        let step = main_space_leave / item_cnt as f32;
        (step / 2.0, step)
      }
      JustifyContent::SpaceBetween => {
        let step = main_space_leave / (item_cnt as f32 - 1.);
        (0., step)
      }
      JustifyContent::SpaceEvenly => {
        let step = main_space_leave / (item_cnt + 1) as f32;
        (step, step)
      }
    }
  }

  pub(crate) fn is_space_layout(&self) -> bool { !matches!(self, JustifyContent::Start) }

  pub(crate) fn is_spacing_distributed(&self) -> bool {
    matches!(
      self,
      JustifyContent::SpaceBetween | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly
    )
  }
}
