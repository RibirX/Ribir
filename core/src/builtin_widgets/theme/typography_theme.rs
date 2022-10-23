use painter::Brush;
use text::FontFamily;

use crate::prelude::BuildCtx;

/// Use typography to present your design and content as clearly and efficiently
/// as possible. The names of the TextTheme properties from the [Material Design
/// spec](https://material.io/design/typography/the-type-system.html#applying-the-type-scale)
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTheme {
  /// Default text font families
  pub default_font_family: Box<[FontFamily]>,
  pub headline1: TextTheme,
  pub headline2: TextTheme,
  pub headline3: TextTheme,
  pub headline4: TextTheme,
  pub headline5: TextTheme,
  pub headline6: TextTheme,
  pub subtitle1: TextTheme,
  pub subtitle2: TextTheme,
  pub body1: TextTheme,
  pub body2: TextTheme,
  pub button: TextTheme,
  pub caption: TextTheme,
  pub overline: TextTheme,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextTheme {
  pub text: painter::TextStyle,
  pub decoration: TextDecorationStyle,
}

/// Encapsulates the text decoration style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationStyle {
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: Brush,
}

bitflags! {
  /// A linear decoration to draw near the text.
  #[derive(Default)]
  pub struct  TextDecoration: u8 {
    const NONE = 0b0001;
    /// Draw a line underneath each line of text
    const UNDERLINE =  0b0010;
    /// Draw a line above each line of text
    const OVERLINE = 0b0100;
    /// Draw a line through each line of text
    const THROUGHLINE = 0b1000;
  }
}

impl TypographyTheme {
  #[inline]
  pub fn of<'a>(ctx: &'a BuildCtx) -> &'a Self { &ctx.theme().typography_theme }
}
