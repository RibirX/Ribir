use self::palette::LightnessCfg;
pub use super::*;
use crate::{fill_icon, prelude::include_svg};
pub use painter::{Brush, Color};

/// Crate a material theme with palette.
pub fn new(brightness: Brightness, palette: Palette) -> Theme {
  let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))]);
  let typography_theme = typography_theme(
    family.clone(),
    family.clone(),
    palette.on_background().into(),
    palette.on_surface_variant().into(),
    TextDecoration::NONE,
    Color::BLACK.with_alpha(0.87).into(),
  );

  let scrollbar = ScrollBarTheme {
    track: ScrollBoxDecorationStyle {
      background: Color::SILVER.into(),
      radius: None,
      thickness: 12.,
    },

    thumb: ScrollBoxDecorationStyle {
      background: Color::GRAY.into(),
      radius: Some(Radius::all(4.)),
      thickness: 12.,
    },
    thumb_min_size: 12.,
  };
  let text_selected_background = TextSelectedBackground {
    focus: Color::from_rgb(50, 150, 255).with_alpha(0.9),
    blur: Color::GRAY.with_alpha(0.9),
  };
  Theme {
    brightness,
    palette,
    typography_theme,
    default_font_family: family,
    scrollbar,
    icon_theme: icon_theme(),
    transitions_theme: TransitionTheme::default(),
    text_selected_background,
    caret_color: Color::BLACK,
  }
}
pub mod purple {
  use super::*;

  fn palette(lightness_cfg: LightnessCfg) -> Palette {
    Palette {
      primary: Color::from_u32(0x6750A4FF),
      secondary: Color::from_u32(0x625B71FF),
      tertiary: Color::from_u32(0x7D5260FF),
      neutral: Color::from_u32(0xFFFBFEFF),
      neutral_variant: Color::from_u32(0xE7E0ECFF),
      error: Color::from_u32(0xB3261EFF),
      warning: Color::from_u32(0xffb74dFF),
      success: Color::from_u32(0x81c784FF),
      lightness_cfg,
    }
  }

  /// A default light blue theme. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn light() -> Theme {
    let palette = palette(LightnessCfg::light_theme_default());
    new(Brightness::Light, palette)
  }

  /// A default dark theme with a teal accent color. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn dark() -> Theme {
    let palette = palette(LightnessCfg::dark_theme_default());
    new(Brightness::Dark, palette)
  }
}

fn icon_theme() -> IconTheme {
  let icon_size = IconSize {
    tiny: Size::new(18., 18.),
    small: Size::new(24., 24.),
    medium: Size::new(36., 36.),
    large: Size::new(48., 48.),
    huge: Size::new(64., 64.),
  };
  let miss_icon = ShareResource::new(include_svg!("./material/miss_icon.svg"));
  let mut theme = IconTheme::new(icon_size, miss_icon);

  fill_icon! { theme,
    icons::CHECKED: "./material/checked.svg",
    icons::UNCHECKED: "./material/unchecked_box.svg",
    icons::INDETERMINATE: "./material/indeterminate_check_box.svg"
  };

  theme
}

/// Create a TypographyTheme which implement the typography styles base on the
/// material design specification.
///
/// The `titles_family` applied to headlines and subtitles and `body_family`
/// applied to body and caption. The `display_style` is applied to
/// headline4, headline3, headline2, headline1, and caption. The
/// `body_style` is applied to the remaining text styles.
pub fn typography_theme(
  titles_family: Box<[FontFamily]>,
  body_family: Box<[FontFamily]>,
  display_style: Brush,
  body_style: Brush,
  decoration: TextDecoration,
  decoration_color: Brush,
) -> TypographyTheme {
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

  TypographyTheme {
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
