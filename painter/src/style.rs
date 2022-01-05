use crate::{Color, FontStretch, FontStyle, FontWeight, ShallowImage};
use algo::CowRc;

// Enum value descriptions are from the CSS spec.
/// A [font family](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#propdef-font-family).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
  /// The name of a font family of choice.
  Name(CowRc<str>),

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
  pub family: Box<[FontFamily]>,
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
#[derive(Clone, PartialEq)]
pub struct TextStyle {
  /// The size of glyphs (in logical pixels) to use when painting the text.
  pub font_size: f32,
  /// The style drawn as a foreground for the text.
  pub foreground: Brush,
  /// The font face to use when painting the text.
  pub font_face: CowRc<FontFace>,
  /// Not support now.
  pub letter_space: f32,
  /// The path style(fill or stroke) to use when painting.
  pub path_style: PathStyle,
}

bitflags::bitflags! {
  /// - Repeat mode repeat the image to full tile the path, if the image greater
  /// than the path, image will be clipped.
  /// - Cover mode resize the image to cover the entire path, even if it has to
  /// stretch the image or cut a little bit off one of the edges
  pub struct TileMode: u8 {
    const REPEAT_X = 0b00000001;
    const REPEAT_Y = 0b00000010;
    const REPEAT_BOTH = Self::REPEAT_X.bits | Self::REPEAT_Y.bits;
    const COVER_X = 0b00000100;
    const COVER_Y = 0b00001000;
    const COVER_BOTH = Self::COVER_X.bits | Self::COVER_Y.bits;
    const REPEAT_X_COVER_Y = Self::REPEAT_X.bits | Self::COVER_Y.bits;
    const COVER_X_REPEAT_Y = Self::COVER_X.bits | Self::REPEAT_Y.bits;
  }
}

impl TileMode {
  #[inline]
  pub fn is_cover_mode(&self) -> bool { self.bits & (TileMode::COVER_BOTH.bits) > 0 }
}

#[derive(Clone, PartialEq)]
pub enum Brush {
  Color(Color),
  Image {
    img: ShallowImage,
    tile_mode: TileMode,
  },
  Gradient, // todo,
}

/// The style to paint path, maybe fill or stroke.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PathStyle {
  /// Fill the path.
  Fill,
  /// Stroke path with line width.
  Stroke(f32),
}

impl Default for FontFace {
  fn default() -> Self {
    Self {
      family: Box::new([FontFamily::Serif]),
      stretch: Default::default(),
      style: Default::default(),
      weight: Default::default(),
    }
  }
}

impl Default for TextStyle {
  fn default() -> Self {
    Self {
      font_size: 14.,
      foreground: Color::BLACK.into(),
      font_face: CowRc::owned(Default::default()),
      letter_space: 0.,
      path_style: PathStyle::Fill,
    }
  }
}

impl From<Color> for Brush {
  #[inline]
  fn from(c: Color) -> Self { Brush::Color(c) }
}

impl Default for Brush {
  #[inline]
  fn default() -> Self { Brush::Color(Color::BLACK) }
}

/// Describe the align in logic without direction.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Align {
  Start,
  Center,
  End,
}

/// Horizontal Alignment
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum HAlign {
  Left,
  Center,
  Right,
}

/// Vertical Alignment
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VALign {
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

impl From<VALign> for Align {
  #[inline]
  fn from(h: VALign) -> Self {
    match h {
      VALign::Top => Align::Start,
      VALign::Center => Align::Center,
      VALign::Bottom => Align::End,
    }
  }
}
