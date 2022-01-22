//! A tiny low level text processing library dedicate to Ribir, use to reorder,
//! shape and do simple layout for text. It's focus
//!
//! Some detail processing learn from [usvg](https://github.com/RazrFalcon/resvg/blob/master/usvg/src/text)
#![feature(test)]
pub mod font_db;
pub mod shaper;
pub use fontdb::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};
pub mod layout;

// Enum value descriptions are from the CSS spec.
/// A [font family](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#propdef-font-family).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
  // todo: no need cow? or directly face ids
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
#[derive(Clone, Debug, PartialEq, Hash)]
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

/// Describe the align in logic without direction.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Align {
  Start,
  Center,
  End,
}

/// Horizontal Alignment
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HAlign {
  Left,
  Center,
  Right,
}

/// Vertical Alignment
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VAlign {
  Top,
  Center,
  Bottom,
}

impl From<HAlign> for Align {
  #[inline]
  fn from(h: HAlign) -> Self {
    match h {
      HAlign::Left => Align::Start,
      HAlign::Center => Align::Center,
      HAlign::Right => Align::End,
    }
  }
}

impl From<VAlign> for Align {
  #[inline]
  fn from(h: VAlign) -> Self {
    match h {
      VAlign::Top => Align::Start,
      VAlign::Center => Align::Center,
      VAlign::Bottom => Align::End,
    }
  }
}
