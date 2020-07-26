pub use canvas::{Color, FillStyle, FontStyle, FontWeight};

#[derive(Clone, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
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

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TextStyle {
  /// The style drawn as a foreground for the text.
  pub foreground: FillStyle,
  /// The name of the font to use when painting the text (e.g., Roboto).
  pub family: String,
  /// The size of glyphs (in logical pixels) to use when painting the text.
  pub font_size: f32,
  /// The typeface variant to use when drawing the letters (e.g., italics).
  pub style: FontStyle,
  /// The typeface thickness to use when painting the text (e.g., bold).
  pub weight: FontWeight,
  // Not support now.
  pub letter_space: f32,
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: Color,
}

/// Use typography to present your design and content as clearly and efficiently
/// as possible. The names of the TextTheme properties from the [Material Design
/// spec](https://material.io/design/typography/the-type-system.html#applying-the-type-scale)
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTheme {
  pub headline1: TextStyle,
  pub headline2: TextStyle,
  pub headline3: TextStyle,
  pub headline4: TextStyle,
  pub headline5: TextStyle,
  pub headline6: TextStyle,
  pub subtitle1: TextStyle,
  pub subtitle2: TextStyle,
  pub body1: TextStyle,
  pub body2: TextStyle,
  pub button: TextStyle,
  pub caption: TextStyle,
  pub overline: TextStyle,
}

/// Properties from [Material Theme](https://material.io/design/material-theming/implementing-your-theme.html)
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeData {
  // Dark or light theme.
  pub brightness: Brightness,
  pub primary: Color,
  pub primary_variant: Color,
  pub secondary: Color,
  pub secondary_variant: Color,
  pub background: Color,
  pub surface: Color,
  pub error: Color,
  pub on_primary: Color,
  pub on_secondary: Color,
  pub on_background: Color,
  pub on_surface: Color,
  pub on_error: Color,
  pub typography_theme: TypographyTheme,
  // Default text font family
  pub default_font_family: String,
}

impl TypographyTheme {
  /// Create a TypographyTheme which implement the typography styles base on the
  /// material design specification.
  ///
  /// The `titles_family` applied to headlines and subtitles and `body_family`
  /// applied to body and caption. The `display_style` is applied to
  /// headline4, headline3, headline2, headline1, and caption. The
  /// `body_style` is applied to the remaining text styles.
  pub fn new(
    titles_family: String,
    body_family: String,
    display_style: FillStyle,
    body_style: FillStyle,
    decoration: TextDecoration,
    decoration_color: Color,
  ) -> Self {
    Self {
      headline1: TextStyle {
        font_size: 96.0,
        weight: FontWeight::LIGHT,
        letter_space: -1.5,
        family: titles_family.clone(),
        foreground: display_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      headline2: TextStyle {
        font_size: 60.0,
        weight: FontWeight::LIGHT,
        letter_space: -0.5,
        family: titles_family.clone(),
        foreground: display_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      headline3: TextStyle {
        font_size: 48.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.0,
        family: titles_family.clone(),
        foreground: display_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      headline4: TextStyle {
        font_size: 34.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.25,
        family: titles_family.clone(),
        foreground: display_style,
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },

      headline5: TextStyle {
        font_size: 24.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.0,
        family: titles_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      headline6: TextStyle {
        font_size: 20.0,
        weight: FontWeight::MEDIUM,
        letter_space: 0.15,
        family: titles_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      subtitle1: TextStyle {
        font_size: 16.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.15,
        family: titles_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      subtitle2: TextStyle {
        font_size: 14.0,
        weight: FontWeight::MEDIUM,
        letter_space: 0.1,
        family: titles_family,
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      body1: TextStyle {
        font_size: 16.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.5,
        family: body_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      body2: TextStyle {
        font_size: 14.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.25,
        family: body_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      button: TextStyle {
        font_size: 14.0,
        weight: FontWeight::MEDIUM,
        letter_space: 1.25,
        family: body_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      caption: TextStyle {
        font_size: 12.0,
        weight: FontWeight::NORMAL,
        letter_space: 0.4,
        family: body_family.clone(),
        foreground: body_style.clone(),
        decoration,
        decoration_color: decoration_color.clone(),
        ..Default::default()
      },
      overline: TextStyle {
        font_size: 10.0,
        weight: FontWeight::NORMAL,
        letter_space: 1.5,
        family: body_family,
        foreground: body_style,
        decoration,
        decoration_color,
        ..Default::default()
      },
    }
  }
}
