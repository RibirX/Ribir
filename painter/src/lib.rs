#![feature(const_fn_floating_point_arithmetic, test)]

//! A 2d logic painter, generate the paint command
pub mod color;
mod painter;
pub mod path;
pub use crate::color::{Color, LightnessTone};

pub use crate::painter::*;
pub use path::*;
pub mod image;
mod style;
pub use image::{PixelImage, ShallowImage};
pub use style::*;
mod svg_parser;
pub use ribir_text;
pub use ribir_text::{typography::Overflow, *};
pub use svg_parser::SvgPaths;

pub use euclid::Transform2D;

pub use lyon_tessellation::{LineCap, LineJoin, StrokeOptions};

pub use ribir_geometry::*;
