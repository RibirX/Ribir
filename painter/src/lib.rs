//! A 2d logic painter, generate the paint command
pub mod color;
mod painter;
pub mod path;
pub mod path_builder;
pub use path::*;

pub use crate::{
  color::{Color, GradientStop, LightnessTone},
  painter::*,
};
pub mod image;
mod style;
pub use style::*;

pub use crate::image::PixelImage;
mod svg;
pub use svg::Svg;
