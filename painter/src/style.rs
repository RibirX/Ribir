use ribir_algo::Resource;
use serde::{Deserialize, Serialize};

use crate::{
  Color, PixelImage,
  color::{LinearGradient, RadialGradient},
};

/// The brush is used to fill or stroke shapes with color, image, or gradient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Brush {
  Color(Color),
  /// Image brush always use a repeat mode to brush the path.
  Image(Resource<PixelImage>),
  RadialGradient(Resource<RadialGradient>),
  LinearGradient(Resource<LinearGradient>),
}

impl Brush {
  /// Returns the color of the brush, or `None` if the brush is not a color.
  pub fn get_color(&self) -> Option<Color> {
    match self {
      Brush::Color(c) => Some(*c),
      _ => None,
    }
  }

  pub fn is_visible(&self) -> bool {
    match self {
      Brush::Color(c) => c.alpha > 0,
      Brush::Image(_) => true,
      Brush::RadialGradient(gradient) => gradient.stops.iter().any(|s| s.color.alpha > 0),
      Brush::LinearGradient(gradient) => gradient.stops.iter().any(|s| s.color.alpha > 0),
    }
  }
}

impl From<Color> for Brush {
  #[inline]
  fn from(c: Color) -> Self { Brush::Color(c) }
}

impl From<Color> for Option<Brush> {
  #[inline]
  fn from(c: Color) -> Self { Some(c.into()) }
}

impl From<Resource<PixelImage>> for Brush {
  #[inline]
  fn from(img: Resource<PixelImage>) -> Self { Brush::Image(img) }
}

impl From<PixelImage> for Brush {
  #[inline]
  fn from(img: PixelImage) -> Self { Resource::new(img).into() }
}

impl From<Resource<RadialGradient>> for Brush {
  #[inline]
  fn from(gradient: Resource<RadialGradient>) -> Self { Brush::RadialGradient(gradient) }
}

impl From<RadialGradient> for Brush {
  #[inline]
  fn from(gradient: RadialGradient) -> Self { Resource::new(gradient).into() }
}

impl From<Resource<LinearGradient>> for Brush {
  #[inline]
  fn from(gradient: Resource<LinearGradient>) -> Self { Brush::LinearGradient(gradient) }
}

impl From<LinearGradient> for Brush {
  #[inline]
  fn from(gradient: LinearGradient) -> Self { Resource::new(gradient).into() }
}

impl Default for Brush {
  #[inline]
  fn default() -> Self { Color::BLACK.into() }
}
