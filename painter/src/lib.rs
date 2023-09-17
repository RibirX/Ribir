#![feature(const_fn_floating_point_arithmetic, test)]

//! A 2d logic painter, generate the paint command
pub mod color;
mod painter;
pub mod path;
pub mod path_builder;
pub use crate::color::{Color, GradientStop, LightnessTone};
pub use crate::painter::*;
pub use path::*;
pub mod image;
mod style;
pub use crate::image::PixelImage;
pub use style::*;
mod svg;
pub use svg::Svg;
