#![feature(const_fn_floating_point_arithmetic, test)]

//! A 2d logic painter, generate the paint command
pub mod color;
mod painter;
pub mod path;
pub use crate::color::*;

pub use crate::painter::*;
pub use path::*;
pub mod image;
mod style;
pub use image::{PixelImage, ShallowImage};
pub use style::*;

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

pub type DeviceRect = euclid::Rect<u32, PhysicUnit>;
pub type DevicePoint = euclid::Point2D<u32, PhysicUnit>;
pub type DeviceSize = euclid::Size2D<u32, PhysicUnit>;
pub type DeviceVector = euclid::Vector2D<u32, PhysicUnit>;

pub use euclid::Transform2D;

pub use lyon_tessellation::{StrokeOptions, LineCap, LineJoin};
