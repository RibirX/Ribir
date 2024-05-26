/// The tag for device unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicUnit;

/// The tag for logic unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicUnit;

pub type Rect<T = f32> = euclid::Rect<T, LogicUnit>;
pub type Point<T = f32> = euclid::Point2D<T, LogicUnit>;
pub type Size<T = f32> = euclid::Size2D<T, LogicUnit>;
pub type Transform<T = f32> = euclid::Transform2D<T, LogicUnit, LogicUnit>;
pub type Vector<T = f32> = euclid::Vector2D<T, LogicUnit>;
pub type Angle<T = f32> = euclid::Angle<T>;
pub type Box2D<T = f32> = euclid::Box2D<T, LogicUnit>;

pub type DeviceRect<T = i32> = euclid::Rect<T, PhysicUnit>;
pub type DevicePoint<T = i32> = euclid::Point2D<T, PhysicUnit>;
pub type DeviceSize<T = i32> = euclid::Size2D<T, PhysicUnit>;
pub type DeviceVector<T = i32> = euclid::Vector2D<T, PhysicUnit>;

pub const INFINITY_SIZE: Size = Size::new(f32::INFINITY, f32::INFINITY);
pub const ZERO_SIZE: Size = Size::new(0., 0.);
use std::ops::Add;

pub use euclid::{num::Zero, rect};

/// Return the four corners of a rectangle: [left-top, right-top,
/// right-bottom, left-bottom]
pub fn rect_corners<T, U>(rect: &euclid::Rect<T, U>) -> [euclid::Point2D<T, U>; 4]
where
  T: Copy + Add<Output = T>,
{
  use euclid::Point2D;

  [
    rect.min(),
    Point2D::new(rect.max_x(), rect.min_y()),
    rect.max(),
    Point2D::new(rect.min_x(), rect.max_y()),
  ]
}

/// Apply the transform to the rect and return the new rect as device unit.
pub fn transform_to_device_rect(rect: &Rect, matrix: &Transform) -> DeviceRect {
  matrix
    .outer_transformed_rect(rect)
    .round_out()
    .to_i32()
    .cast_unit()
}
