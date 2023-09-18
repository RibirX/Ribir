use crate::{
  color::{LinearGradient, RadialGradient},
  Color, PixelImage,
};
use ribir_algo::ShareResource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Brush {
  Color(Color),
  /// Image brush always use a repeat mode to brush the path.
  Image(ShareResource<PixelImage>),
  RadialGradient(RadialGradient),
  LinearGradient(LinearGradient),
}

impl Brush {
  pub fn only_convert_color(&self, f: impl FnOnce(&Color) -> Color) -> Brush {
    match self {
      Brush::Color(color) => f(color).into(),
      _ => panic!("Need Color!"),
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

impl From<ShareResource<PixelImage>> for Brush {
  #[inline]
  fn from(img: ShareResource<PixelImage>) -> Self { Brush::Image(img) }
}

impl From<PixelImage> for Brush {
  #[inline]
  fn from(img: PixelImage) -> Self { ShareResource::new(img).into() }
}

impl Default for Brush {
  #[inline]
  fn default() -> Self { Color::BLACK.into() }
}
