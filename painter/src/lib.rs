#![allow(clippy::needless_lifetimes)]

//! A 2d logic painter, generate the paint command
pub mod color;
pub mod filter;
mod painter;
pub mod path;
pub mod path_builder;
pub use path::*;
mod text;
pub use text::*;

pub use crate::{
  color::{Color, GradientStop, LightnessTone},
  filter::*,
  painter::*,
};
pub mod image;
mod style;
pub use style::*;

pub use crate::image::PixelImage;
mod svg;
pub use svg::Svg;
