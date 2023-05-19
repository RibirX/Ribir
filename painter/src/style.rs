use crate::{Color, PixelImage};
use ribir_algo::ShareResource;
use ribir_text::{Em, FontFace, FontSize, Pixel};
use serde::{Deserialize, Serialize};

/// Encapsulates the text style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
  /// The size of glyphs (in logical pixels) to use when painting the text.
  pub font_size: FontSize,
  /// The font face to use when painting the text.
  // todo: use ids instead of
  pub font_face: FontFace,
  /// Not support now.
  pub letter_space: Option<Pixel>,
  /// The factor use to multiplied by the font size to specify the text line
  /// height.
  pub line_height: Option<Em>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Brush {
  Color(Color),
  /// Image brush always use a repeat mode to brush the path.
  Image(ShareResource<PixelImage>),
  Gradient, // todo,
}

impl Brush {
  pub fn only_convert_color(&self, f: impl FnOnce(&Color) -> Color) -> Brush {
    match self {
      Brush::Color(color) => f(color).into(),
      _ => panic!("Need Color!"),
    }
  }
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_size: FontSize::Pixel(14.0.into()),
      font_face: Default::default(),
      letter_space: None,
      line_height: None,
    }
  }
}

impl From<Color> for Brush {
  #[inline]
  fn from(c: Color) -> Self { Brush::Color(c) }
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
