#![feature(decl_macro)]
mod atlas;
mod canvas;
mod ctx_2d;
mod layer_2d;

pub use crate::canvas::*;
pub use layer_2d::*;

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

pub type PhysicRect = euclid::Rect<u32, PhysicUnit>;
pub type PhysicPoint = euclid::Point2D<u32, PhysicUnit>;
pub type PhysicSize = euclid::Size2D<u32, PhysicUnit>;
