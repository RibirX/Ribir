use crate::{Color, ShallowImage};
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

bitflags::bitflags! {
  /// - Repeat mode repeat the image to full tile the path, if the image greater
  /// than the path, image will be clipped.
  /// - Cover mode resize the image to cover the entire path, even if it has to
  /// stretch the image or cut a little bit off one of the edges
  #[derive(PartialEq, Debug, Clone, Copy)]
  pub struct TileMode: u8 {
    const REPEAT_X = 0b00000001;
    const REPEAT_Y = 0b00000010;
    const REPEAT_BOTH = Self::REPEAT_X.bits() | Self::REPEAT_Y.bits();
    const COVER_X = 0b00000100;
    const COVER_Y = 0b00001000;
    const COVER_BOTH = Self::COVER_X.bits() | Self::COVER_Y.bits();
    const REPEAT_X_COVER_Y = Self::REPEAT_X.bits() | Self::COVER_Y.bits();
    const COVER_X_REPEAT_Y = Self::COVER_X.bits() | Self::REPEAT_Y.bits();
  }
}

macro_rules! impl_bitflags_serde {
  ($ty: ident, $name: expr) => {
    impl serde::Serialize for $ty {
      fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        bitflags_serde_legacy::serialize(self, $name, serializer)
      }
    }

    impl<'de> serde::Deserialize<'de> for $ty {
      fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bitflags_serde_legacy::deserialize($name, deserializer)
      }
    }
  };
}

impl_bitflags_serde!(TileMode, "TileMode");

impl TileMode {
  #[inline]
  pub fn is_cover_mode(&self) -> bool { self.bits() & (TileMode::COVER_BOTH.bits()) > 0 }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Brush {
  Color(Color),
  Image {
    img: ShallowImage,
    tile_mode: TileMode,
    opacify: f32,
  },
  Gradient, // todo,
}

impl Brush {
  pub fn only_convert_color(&self, f: impl FnOnce(&Color) -> Color) -> Brush {
    match self {
      Brush::Color(color) => f(color).into(),
      _ => self.clone(),
    }
  }

  pub fn apply_opacify(&mut self, alpha: f32) {
    match self {
      Brush::Color(c) => *c = c.apply_alpha(alpha),
      Brush::Image { opacify, .. } => *opacify *= alpha,
      Brush::Gradient => todo!(),
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

impl Default for Brush {
  #[inline]
  fn default() -> Self { Color::BLACK.into() }
}
