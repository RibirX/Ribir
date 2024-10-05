//! A tiny low level text processing library dedicate to Ribir, use to reorder,
//! shape and do simple layout for text. It's focus
//!
//! Some detail processing learn from [usvg](https://github.com/RazrFalcon/resvg/blob/master/usvg/src/text)
pub mod font_db;
pub mod shaper;
use std::hash::Hash;

use derive_more::{Add, AddAssign, Mul, Neg, Sub, SubAssign};
use font_db::Face;
pub use fontdb::{ID, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};
pub use ribir_algo::Substr;
use ribir_geom::{Rect, rect};
use rustybuzz::{GlyphPosition, ttf_parser::GlyphId};
pub mod text_reorder;
pub mod typography;
pub use text_reorder::TextReorder;
mod typography_store;
pub use typography_store::{TypographyStore, VisualGlyphs};
mod svg_glyph_cache;

mod text_writer;
pub use text_writer::{
  CharacterCursor, TextWriter, select_next_word, select_prev_word, select_word,
};

mod grapheme_cursor;
pub use grapheme_cursor::GraphemeCursor;

pub mod unicode_help;

// Enum value descriptions are from the CSS spec.
/// A [font family](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#propdef-font-family).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
  /// The name of a font family of choice.
  Name(std::borrow::Cow<'static, str>),

  /// Serif fonts represent the formal text style for a script.
  Serif,

  /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low
  /// contrast and have stroke endings that are plain — without any flaring,
  /// cross stroke, or other ornamentation.
  SansSerif,

  /// Glyphs in cursive fonts generally use a more informal script style,
  /// and the result looks more like handwritten pen or brush writing than
  /// printed letterwork.
  Cursive,

  /// Fantasy fonts are primarily decorative or expressive fonts that
  /// contain decorative or expressive representations of characters.
  Fantasy,

  /// The sole criterion of a monospace font is that all glyphs have the same
  /// fixed width.
  Monospace,
}

/// Encapsulates the font properties of font face.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontFace {
  /// A prioritized list of font family names or generic family names.
  ///
  /// [font-family](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#propdef-font-family) in CSS.
  pub families: Box<[FontFamily]>,
  /// Selects a normal, condensed, or expanded face from a font family.
  ///
  /// [font-stretch](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#font-stretch-prop) in CSS.
  pub stretch: FontStretch,
  /// Allows italic or oblique faces to be selected.
  ///
  /// [font-style](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#font-style-prop) in CSS.
  pub style: FontStyle,
  /// Specifies the weight of glyphs in the font, their degree of blackness or
  /// stroke thickness.
  ///
  /// [font-weight](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#font-weight-prop) in CSS.
  pub weight: FontWeight,
}

/// Encapsulates the text style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
  /// The size of fonts (in logical pixels) to use when painting the text.
  pub font_size: f32,
  /// The font face to use when painting the text.
  pub font_face: FontFace,
  /// The space between characters in logical pixel units.
  pub letter_space: f32,
  /// The line height of the text in logical pixels.
  pub line_height: f32,
  /// How to handle the visual overflow.
  pub overflow: Overflow,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum Overflow {
  #[default]
  Clip,
  AutoWrap,
}

impl Overflow {
  fn is_auto_wrap(&self) -> bool { matches!(self, Overflow::AutoWrap) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
  /// The font face id of the glyph.
  pub face_id: ID,
  /// How many units the line advances after drawing this glyph when setting
  /// text in horizontal direction.
  pub x_advance: GlyphUnit,
  /// How many units the line advances after drawing this glyph when setting
  /// text in vertical direction.
  pub y_advance: GlyphUnit,
  /// How many units the glyph moves on the X-axis before drawing it, this
  /// should not affect how many the line advances.
  pub x_offset: GlyphUnit,
  /// How many units the glyph moves on the Y-axis before drawing it, this
  /// should not affect how many the line advances.
  pub y_offset: GlyphUnit,
  /// The id of the glyph.
  pub glyph_id: GlyphId,
  /// An cluster of origin text as byte index.
  pub cluster: u32,
}

#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Add, Sub, AddAssign, Mul, SubAssign,
  Neg, Hash
)]
pub struct GlyphUnit(i32);

impl Default for FontFace {
  fn default() -> Self {
    Self {
      families: Box::new([FontFamily::Serif]),
      stretch: Default::default(),
      style: Default::default(),
      weight: Default::default(),
    }
  }
}

/// Text-align relative to the horizontal or vertical, not caring about whether
/// the text is left-to-right or right-to-left, In the horizontal the left is
/// the start, and in vertical the top is the start.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TextAlign {
  Start,
  Center,
  End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextDirection {
  /// Text is set horizontally from left to right.
  LeftToRight,
  /// Text is set horizontally from right to left.
  RightToLeft,
  /// Text is set vertically from top to bottom.
  TopToBottom,
  /// Text is set vertically from bottom to top.
  BottomToTop,
}

impl TextDirection {
  #[inline]
  pub fn is_vertical(&self) -> bool {
    matches!(self, TextDirection::TopToBottom | TextDirection::BottomToTop)
  }

  #[inline]
  pub fn is_horizontal(&self) -> bool {
    matches!(self, TextDirection::LeftToRight | TextDirection::RightToLeft)
  }
}

impl GlyphUnit {
  /// Specifies the standard units per EM of the glyph.
  pub const UNITS_PER_EM: u16 = 16384;
  /// Specifies the pixels for a standard EM.
  pub const PIXELS_PER_EM: u16 = 16;
  /// Specifies the units of a pixel of a standard EM.
  pub const UNITS_PER_PIXEL: u16 = Self::UNITS_PER_EM / Self::PIXELS_PER_EM;

  pub const ZERO: Self = Self(0);
  pub const MAX: Self = Self(i32::MAX);
  pub const STANDARD_EM: Self = Self(Self::UNITS_PER_EM as i32);

  pub fn from_pixel(pos: f32) -> Self { Self(f32::ceil(pos * Self::UNITS_PER_PIXEL as f32) as i32) }

  pub fn max(&self, other: Self) -> Self { Self(self.0.max(other.0)) }

  pub fn min(&self, other: Self) -> Self { Self(self.0.min(other.0)) }

  /// Render a standard font glyph at a different font size.
  pub fn cast_to(self, pixel_per_em: f32) -> Self {
    let scale = pixel_per_em / GlyphUnit::PIXELS_PER_EM as f32;
    cast(self.0, scale)
  }

  pub fn into_pixel(self) -> f32 { self.0 as f32 / Self::UNITS_PER_PIXEL as f32 }
}
fn cast(pos: i32, scale: f32) -> GlyphUnit { GlyphUnit(f32::ceil(pos as f32 * scale) as i32) }

impl Glyph {
  fn new(glyph_id: GlyphId, cluster: u32, pos: &GlyphPosition, face: &Face) -> Self {
    let scale = GlyphUnit::UNITS_PER_EM as f32 / face.units_per_em() as f32;
    Glyph {
      face_id: face.face_id,
      x_advance: cast(pos.x_advance, scale),
      y_advance: cast(pos.y_advance, scale),
      x_offset: cast(pos.x_offset, scale),
      y_offset: cast(pos.y_offset, scale),
      glyph_id,
      cluster,
    }
  }
  pub fn is_miss(&self) -> bool { self.glyph_id.0 == 0 }

  #[allow(unused)]
  pub fn is_not_miss(&self) -> bool { !self.is_miss() }

  /// Cast the standard font size glyph to the specify font size.
  pub fn cast_to(mut self, pixel_per_em: f32) -> Self {
    let scale = pixel_per_em / GlyphUnit::PIXELS_PER_EM as f32;

    self.x_advance = cast(self.x_advance.0, scale);
    self.y_advance = cast(self.y_advance.0, scale);
    self.x_offset = cast(self.x_offset.0, scale);
    self.y_offset = cast(self.y_offset.0, scale);
    self
  }

  pub fn bounds(&self) -> Rect {
    rect(
      self.x_offset.into_pixel(),
      self.y_offset.into_pixel(),
      self.x_advance.into_pixel(),
      self.y_advance.into_pixel(),
    )
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
      line_height: 16.,
      overflow: <_>::default(),
    }
  }
}
