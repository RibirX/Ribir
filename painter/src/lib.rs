#![feature(const_fn_floating_point_arithmetic, test)]

//! A 2d logic painter, generate the paint command
pub mod color;
mod painter;
pub mod path;
pub mod path_builder;
pub use crate::color::{Color, LightnessTone};
pub use crate::painter::*;
pub use path::*;
pub mod image;
mod style;
pub use image::PixelImage;
pub use style::*;
mod svg;
pub use ribir_text;
pub use ribir_text::{typography::Overflow, *};
pub use svg::Svg;

/// The tag for device unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicUnit;

/// The tag for logic unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicUnit;

pub type Rect = euclid::Rect<f32, LogicUnit>;
pub type Point = euclid::Point2D<f32, LogicUnit>;
pub type Size = euclid::Size2D<f32, LogicUnit>;
pub type Transform = euclid::Transform2D<f32, LogicUnit, LogicUnit>;
pub type Vector = euclid::Vector2D<f32, LogicUnit>;
pub type Angle = euclid::Angle<f32>;
pub type Box2D = euclid::Box2D<f32, LogicUnit>;

pub type DeviceRect = euclid::Rect<i32, PhysicUnit>;
pub type DevicePoint = euclid::Point2D<i32, PhysicUnit>;
pub type DeviceSize = euclid::Size2D<i32, PhysicUnit>;
pub type DeviceVector = euclid::Vector2D<i32, PhysicUnit>;
pub use euclid::Transform2D;
pub const INFINITY_SIZE: Size = Size::new(f32::INFINITY, f32::INFINITY);
pub const ZERO_SIZE: Size = Size::new(0., 0.);

pub mod geom {
  pub use euclid::rect;
  use std::ops::Add;

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
}
