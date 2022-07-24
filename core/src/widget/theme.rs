//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.
pub mod material;
mod palette;
pub use palette::Palette;

use crate::{
  impl_proxy_query, impl_query_self_only,
  prelude::{
    compose_child_as_data_widget, Any, BuildCtx, ComposeSingleChild, Declare, DeclareBuilder,
    Query, QueryFiler, QueryOrder, Stateful, TypeId, Widget,
  },
};
use algo::ShareResource;
pub use painter::*;
use text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

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

/// Encapsulates the text decoration style for painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDecorationStyle {
  /// The decorations to paint near the text
  pub decoration: TextDecoration,
  /// The color in which to paint the text decorations.
  pub decoration_color: Brush,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextTheme {
  pub text: TextStyle,
  pub decoration: TextDecorationStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBoxDecorationStyle {
  pub background: Brush,

  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.]
  pub radius: Option<Radius>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBarTheme {
  pub track_box: ScrollBoxDecorationStyle,
  pub track_width: f32,

  pub thumb_box: ScrollBoxDecorationStyle,
  pub thumb_width: f32,
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

#[derive(Clone, Debug)]
pub struct Theme {
  // Dark or light theme.
  pub brightness: Brightness,
  pub palette: Palette,
  pub typography_theme: TypographyTheme,
  /// Default text font families
  pub default_font_family: Box<[FontFamily]>,
  pub scrollbar: ScrollBarTheme,
  pub icon_theme: IconTheme,
}

#[derive(Debug, Clone)]
pub struct IconTheme {
  pub icon_size: IconSize,
  pub builtin_icons: SvgIcons,
}

#[derive(Debug, Clone)]
pub struct IconSize {
  pub tiny: Size,
  pub small: Size,
  pub medium: Size,
  pub large: Size,
  pub huge: Size,
}

impl IconSize {
  #[inline]
  pub fn of<'a>(ctx: &'a mut BuildCtx) -> &'a Self { &ctx.theme().icon_theme.icon_size }
}

impl SvgIcons {
  #[inline]
  pub fn of<'a>(ctx: &'a mut BuildCtx) -> &'a Self { &ctx.theme().icon_theme.builtin_icons }
}
#[derive(Debug, Clone)]
pub struct SvgIcons {
  pub checked: ShareResource<SvgRender>,
  pub unchecked: ShareResource<SvgRender>,
  pub indeterminate: ShareResource<SvgRender>,
}
#[derive(Declare)]
pub struct ThemeWidget {
  #[declare(builtin)]
  pub theme: Theme,
}

impl ComposeSingleChild for ThemeWidget {
  #[inline]
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    // todo: theme can provide fonts to load.
    compose_child_as_data_widget(child, this, |w| w.theme)
  }
}

impl Query for Theme {
  impl_query_self_only!();
}

impl Query for ThemeWidget {
  impl_proxy_query!(theme);
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
    display_style: Brush,
    body_style: Brush,
    decoration: TextDecoration,
    decoration_color: Brush,
  ) -> Self {
    let decoration = TextDecorationStyle { decoration, decoration_color };
    let light_title_face = FontFace {
      families: titles_family,
      weight: FontWeight::LIGHT,
      ..<_>::default()
    };

    let mut normal_title_face = light_title_face.clone();
    normal_title_face.weight = FontWeight::NORMAL;

    let mut medium_title_face = light_title_face.clone();
    medium_title_face.weight = FontWeight::MEDIUM;

    let body_face = FontFace {
      families: body_family,
      ..<_>::default()
    };

    Self {
      headline1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(96.0.into()),
          letter_space: Some(Pixel::from(-1.5)),
          foreground: display_style.clone(),
          font_face: light_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(60.0.into()),
          letter_space: Some(Pixel::from(-0.5)),
          foreground: display_style.clone(),
          font_face: light_title_face,
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline3: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(48.0.into()),
          foreground: display_style.clone(),
          letter_space: Some(Pixel(0.0.into())),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      headline4: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(34.0.into()),
          foreground: display_style.clone(),
          letter_space: Some(Pixel(0.25.into())),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline5: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(24.0.into()),
          letter_space: Some(Pixel(0.0.into())),
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      headline6: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(20.0.into()),
          letter_space: Some(Pixel(0.15.into())),
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      subtitle1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(16.0.into()),
          letter_space: Some(Pixel(0.15.into())),
          foreground: body_style.clone(),
          font_face: normal_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      subtitle2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(0.1.into())),
          foreground: body_style.clone(),
          font_face: medium_title_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      body1: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(16.0.into()),
          letter_space: Some(Pixel(0.5.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },

      body2: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(0.25.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      button: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(14.0.into()),
          letter_space: Some(Pixel(1.25.into())),
          foreground: body_style.clone(),
          font_face: {
            let mut face = body_face.clone();
            face.weight = FontWeight::MEDIUM;
            face
          },
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      caption: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(12.0.into()),
          letter_space: Some(Pixel(0.4.into())),
          foreground: body_style.clone(),
          font_face: body_face.clone(),
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration: decoration.clone(),
      },
      overline: TextTheme {
        text: TextStyle {
          font_size: FontSize::Pixel(10.0.into()),
          letter_space: Some(Pixel(1.5.into())),
          foreground: body_style,
          font_face: body_face,
          path_style: PathStyle::Fill,
          line_height: None,
        },
        decoration,
      },
    }
  }
}
