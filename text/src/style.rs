use derive_more::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use crate::{FontFace, FontRequest};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
  Scale(f32),
  Px(f32),
}

impl LineHeight {
  #[inline]
  pub fn resolve_for_font_size(self, font_size: f32) -> f32 {
    match self {
      LineHeight::Scale(value) => font_size * value,
      LineHeight::Px(value) => value,
    }
  }
}

impl Default for LineHeight {
  #[inline]
  fn default() -> Self { Self::Scale(1.2) }
}

impl From<f32> for LineHeight {
  #[inline]
  fn from(value: f32) -> Self { Self::Px(value) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextWrap {
  #[default]
  NoWrap,
  Wrap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphStyle {
  pub text_align: TextAlign,
  pub wrap: TextWrap,
}

impl Default for ParagraphStyle {
  fn default() -> Self { Self { text_align: TextAlign::Start, wrap: TextWrap::default() } }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpanStyle<Brush> {
  pub font: Option<FontRequest>,
  pub font_size: Option<f32>,
  pub letter_spacing: Option<f32>,
  pub line_height: Option<LineHeight>,
  pub brush: Option<Brush>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
  pub font_size: f32,
  pub font_face: FontFace,
  pub letter_space: f32,
  pub line_height: LineHeight,
  pub overflow: TextOverflow,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum TextOverflow {
  #[default]
  Overflow,
  AutoWrap,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextAlign {
  #[default]
  Start,
  Center,
  End,
}

#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Add, Sub, AddAssign, Mul, SubAssign,
  Neg, Hash
)]
pub struct GlyphUnit(i32);

impl GlyphUnit {
  pub const UNITS_PER_EM: u16 = 16384;
  pub const PIXELS_PER_EM: u16 = 16;
  pub const UNITS_PER_PIXEL: u16 = Self::UNITS_PER_EM / Self::PIXELS_PER_EM;

  pub const ZERO: Self = Self(0);
  pub const MAX: Self = Self(i32::MAX);
  pub const STANDARD_EM: Self = Self(Self::UNITS_PER_EM as i32);

  pub fn new(pos: i32) -> Self { Self(pos) }

  pub fn from_pixel(pos: f32) -> Self { Self(f32::ceil(pos * Self::UNITS_PER_PIXEL as f32) as i32) }

  pub fn max(&self, other: Self) -> Self { Self(self.0.max(other.0)) }

  pub fn min(&self, other: Self) -> Self { Self(self.0.min(other.0)) }

  pub fn cast_to(self, pixel_per_em: f32) -> Self {
    let scale = pixel_per_em / GlyphUnit::PIXELS_PER_EM as f32;
    cast(self.0, scale)
  }

  pub fn into_pixel(self) -> f32 { self.0 as f32 / Self::UNITS_PER_PIXEL as f32 }
}

fn cast(pos: i32, scale: f32) -> GlyphUnit { GlyphUnit(f32::ceil(pos as f32 * scale) as i32) }

impl TextStyle {
  #[inline]
  pub fn with_font_size(self, font_size: f32) -> Self { Self { font_size, ..self } }

  #[inline]
  pub fn with_font_face(self, font_face: FontFace) -> Self { Self { font_face, ..self } }

  #[inline]
  pub fn with_letter_space(self, letter_space: f32) -> Self { Self { letter_space, ..self } }

  #[inline]
  pub fn with_line_height(self, line_height: impl Into<LineHeight>) -> Self {
    Self { line_height: line_height.into(), ..self }
  }

  #[inline]
  pub fn with_overflow(self, overflow: TextOverflow) -> Self { Self { overflow, ..self } }
}

pub fn single_style_paragraph_style(
  text_style: &TextStyle, text_align: TextAlign,
) -> ParagraphStyle {
  ParagraphStyle {
    text_align,
    wrap: match text_style.overflow {
      TextOverflow::Overflow => TextWrap::NoWrap,
      TextOverflow::AutoWrap => TextWrap::Wrap,
    },
  }
}

pub fn single_style_span_style<Brush>(text_style: &TextStyle) -> SpanStyle<Brush> {
  SpanStyle {
    font: Some(crate::FontRequest { face: text_style.font_face.clone() }),
    font_size: Some(text_style.font_size),
    letter_spacing: Some(text_style.letter_space),
    line_height: None,
    brush: None,
  }
}

impl std::ops::Div<f32> for GlyphUnit {
  type Output = GlyphUnit;

  #[inline]
  fn div(self, rhs: f32) -> Self::Output { cast(self.0, 1. / rhs) }
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_size: 14.,
      font_face: Default::default(),
      letter_space: 0.,
      line_height: LineHeight::default(),
      overflow: <_>::default(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_line_height_is_scale_1_2() {
    assert_eq!(LineHeight::default(), LineHeight::Scale(1.2));
    assert_eq!(TextStyle::default().line_height, LineHeight::Scale(1.2));
  }

  #[test]
  fn bare_f32_maps_to_px_line_height() {
    assert_eq!(LineHeight::from(24.), LineHeight::Px(24.));
  }
}
