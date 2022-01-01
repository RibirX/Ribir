//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.
pub mod material;

pub use canvas::{Color, FillStyle, Path, PathBuilder, Point};
pub use fontdb::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};

use super::CowRc;

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
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
  /// The size of glyphs (in logical pixels) to use when painting the text.
  pub font_size: f32,
  /// The style drawn as a foreground for the text.
  pub foreground: FillStyle,
  /// The font face to use when painting the text.
  pub font_face: CowRc<FontFace>,
  // Not support now.
  pub letter_space: f32,
}

/// Encapsulates the text decoration style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationStyle {
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: FillStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextTheme {
  pub text: TextStyle,
  pub decoration: TextDecorationStyle,
}
/// Use typography to present your design and content as clearly and efficiently
/// as possible. The names of the TextTheme properties from the [Material Design
/// spec](https://material.io/design/typography/the-type-system.html#applying-the-type-scale)
#[derive(Clone, Debug, PartialEq)]
pub struct TypographyTheme {
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

/// Properties from [Material Theme](https://material.io/design/material-theming/implementing-your-theme.html)
#[derive(Clone, Debug)]
pub struct Theme {
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
  /// The color used for widgets in their inactive (but enabled) state.
  pub unselected_widget_color: Color,
  /// Default text font family
  pub default_font_family: Box<[FontFamily]>,
  pub checkbox: CheckboxTheme,
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
    titles_family: Box<[FontFamily]>,
    body_family: Box<[FontFamily]>,
    display_style: FillStyle,
    body_style: FillStyle,
    decoration: TextDecoration,
    decoration_color: FillStyle,
  ) -> Self {
    let decoration = TextDecorationStyle { decoration, decoration_color };
    let light_title_face: CowRc<FontFace> = CowRc::owned(FontFace {
      family: titles_family,
      weight: FontWeight::LIGHT,
      ..<_>::default()
    });

    let mut normal_title_face = light_title_face.clone();
    normal_title_face.to_mut().weight = FontWeight::NORMAL;

    let mut medium_title_face = light_title_face.clone();
    medium_title_face.to_mut().weight = FontWeight::MEDIUM;

    let body_face: CowRc<FontFace> = CowRc::owned(FontFace {
      family: body_family,
      ..<_>::default()
    });

    Self {
      headline1: TextTheme {
        text: TextStyle {
          font_size: 96.0,
          letter_space: -1.5,
          foreground: display_style.clone(),
          font_face: light_title_face.clone(),
        },
        decoration: decoration.clone(),
      },
      headline2: TextTheme {
        text: TextStyle {
          font_size: 60.0,
          letter_space: -0.5,
          foreground: display_style.clone(),
          font_face: light_title_face,
        },
        decoration: decoration.clone(),
      },
      headline3: TextTheme {
        text: TextStyle {
          font_size: 48.0,
          foreground: display_style.clone(),
          letter_space: 0.0,
          font_face: normal_title_face.clone(),
        },
        decoration: decoration.clone(),
      },

      headline4: TextTheme {
        text: TextStyle {
          font_size: 34.0,
          foreground: display_style.clone(),
          letter_space: 0.25,
          font_face: normal_title_face.clone(),
        },
        decoration: decoration.clone(),
      },
      headline5: TextTheme {
        text: TextStyle {
          font_size: 24.0,
          letter_space: 0.0,
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
        },
        decoration: decoration.clone(),
      },
      headline6: TextTheme {
        text: TextStyle {
          font_size: 20.0,
          letter_space: 0.15,
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
        },
        decoration: decoration.clone(),
      },

      subtitle1: TextTheme {
        text: TextStyle {
          font_size: 16.0,
          letter_space: 0.15,
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
        },
        decoration: decoration.clone(),
      },
      subtitle2: TextTheme {
        text: TextStyle {
          font_size: 14.0,
          letter_space: 0.1,
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
        },
        decoration: decoration.clone(),
      },
      body1: TextTheme {
        text: TextStyle {
          font_size: 16.0,
          letter_space: 0.5,
          foreground: body_style.clone(),
          font_face: body_face.clone(),
        },
        decoration: decoration.clone(),
      },

      body2: TextTheme {
        text: TextStyle {
          font_size: 14.0,
          letter_space: 0.25,
          foreground: body_style.clone(),
          font_face: body_face.clone(),
        },
        decoration: decoration.clone(),
      },
      button: TextTheme {
        text: TextStyle {
          font_size: 14.0,
          letter_space: 1.25,
          foreground: body_style.clone(),
          font_face: {
            let mut face = body_face.clone();
            face.to_mut().weight = FontWeight::MEDIUM;
            face
          },
        },
        decoration: decoration.clone(),
      },
      caption: TextTheme {
        text: TextStyle {
          font_size: 12.0,
          letter_space: 0.4,
          foreground: body_style.clone(),
          font_face: body_face.clone(),
        },
        decoration: decoration.clone(),
      },
      overline: TextTheme {
        text: TextStyle {
          font_size: 10.0,
          letter_space: 1.5,
          foreground: body_style,
          font_face: body_face,
        },
        decoration,
      },
    }
  }
}

// todo: more general
#[derive(Debug, Clone)]
pub struct CheckboxTheme {
  pub size: f32,
  pub border_width: f32,
  pub check_mark_width: f32,
  pub color: Color,
  pub border_radius: f32,
  pub border_color: Color,
  pub marker_color: Color,
  pub checked_path: Path,
  pub indeterminate_path: Path,
}

impl Default for CheckboxTheme {
  fn default() -> Self {
    let size: f32 = 12.;
    let border_width = 2.;
    let checked_path = {
      let mut builder = PathBuilder::new();
      let start = Point::new(2.733_333_3, 8.466_667);
      let mid = Point::new(6., 11.733_333);
      let end = Point::new(13.533_333, 4.2);
      builder.segment(start, mid).segment(mid, end);
      builder.build()
    };

    let center_y = size / 2. + border_width;
    let indeterminate_path = {
      let mut builder = PathBuilder::new();
      builder
        .begin_path(Point::new(3., center_y))
        .line_to(Point::new(size + border_width * 2. - 3., center_y))
        .close_path();
      builder.build()
    };

    Self {
      size,
      border_width,
      check_mark_width: 1.422_222,
      marker_color: Color::WHITE,
      color: Color::BLACK,
      border_radius: 2.,
      border_color: Color::BLACK,
      checked_path,
      indeterminate_path,
    }
  }
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
    }
  }
}
