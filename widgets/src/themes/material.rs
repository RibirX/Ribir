pub use super::*;
use crate::prelude::*;
use ribir_core::{fill_svgs, prelude::*};
pub mod ripple;
pub mod state_layer;
pub use ripple::*;
pub use state_layer::*;

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

  let text_selected_background = TextSelectedBackground {
    focus: Color::from_rgb(50, 150, 255).with_alpha(0.9),
    blur: Color::GRAY.with_alpha(0.9),
  };

  let mut theme = FullTheme {
    brightness,
    palette,
    typography_theme,
    icon_theme: icon_theme(),
    transitions_theme: TransitionTheme::default(),
    text_selected_background,
    caret_color: Color::BLACK,
    compose_styles: <_>::default(),
    custom_themes: <_>::default(),
  };

  fill_svgs! { theme.icon_theme,
    svgs::ADD_CIRCLE: "./material/icons/add_circle_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ADD: "./material/icons/add_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_BACK: "./material/icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_DROP_DOWN: "./material/icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_FORWARD: "./material/icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CANCEL: "./material/icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX: "./material/icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX_OUTLINE_BLANK: "./material/icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_CIRCLE: "./material/icons/check_circle_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK: "./material/icons/check_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHEVRON_RIGHT: "./material/icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CLOSE: "./material/icons/close_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DELETE: "./material/icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DONE: "./material/icons/done_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::EXPAND_MORE: "./material/icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FAVORITE: "./material/icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FILE_DOWNLOAD: "./material/icons/file_download_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::GRADE: "./material/icons/grade_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::HOME: "./material/icons/home_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::INDETERMINATE_CHECK_BOX: "./material/icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGIN: "./material/icons/login_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGOUT: "./material/icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MENU: "./material/icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_VERT: "./material/icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::REFRESH: "./material/icons/refresh_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SEARCH: "./material/icons/search_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SETTINGS: "./material/icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::STAR: "./material/icons/star_FILL0_wght400_GRAD0_opsz48.svg"
  };

  override_compose_style(&mut theme);
  init_custom_theme(&mut theme);
  Theme::Full(theme)
}

fn init_custom_theme(theme: &mut FullTheme) {
  theme.custom_themes.set_custom_theme(ScrollBarTheme {
    thumb_min_size: 12.,
    thickness: 8.,
    track_brush: theme.palette.primary_container().into(),
  });
  theme.custom_themes.set_custom_theme(CheckBoxTheme {
    size: theme.icon_theme.icon_size.tiny,
    label_style: theme.typography_theme.body1.text.clone(),
  });
  theme.custom_themes.set_custom_theme(ButtonTheme {
    padding: 4.,
    radius: 4.,
    border_color: theme.palette.primary(),
    background: theme.palette.primary(),
    foreground: theme.palette.surface(),
  });
}

fn override_compose_style(theme: &mut FullTheme) {
  fn scrollbar_thumb(host: Widget, margin: EdgeInsets) -> Widget {
    widget! {
      DynWidget {
        dyns: host,
        margin,
        border_radius: Radius::all(4.),
        background: Palette::of(ctx).primary()
      }
    }
  }

  let styles = &mut theme.compose_styles;
  styles.override_compose_style::<HScrollBarThumbStyle>(|this, host| {
    widget! {
      states { this }
      DynWidget {
        id: thumb,
        left_anchor: this.offset,
        dyns: scrollbar_thumb(host, EdgeInsets::vertical(1.))
      }

      transition prop!(thumb.left_anchor, PositionUnit::lerp_fn(thumb.layout_width())) {
        by: transitions::SMOOTH_SCROLL.of(ctx),
      }
    }
  });
  styles.override_compose_style::<VScrollBarThumbStyle>(|this, host| {
    widget! {
      states { this }
      DynWidget {
        id: thumb,
        top_anchor: this.offset,
        dyns: scrollbar_thumb(host, EdgeInsets::vertical(1.))
      }

      transition prop!(thumb.top_anchor, PositionUnit::lerp_fn(thumb.layout_height())) {
        by: transitions::SMOOTH_SCROLL.of(ctx),
      }
    }
  });
  styles.override_compose_style::<InkBarStyle>(|style, _| {
    widget! {
      states { style }
      init { let palette = Palette::of(ctx); }
      Container {
        id: ink_bar,
        size: Size::new(style.ink_bar_rect.size.width, 2.),
        left_anchor: style.ink_bar_rect.origin.x,
        top_anchor: style.ink_bar_rect.size.height - 2.,
        background: palette.primary(),
      }

      transition prop!(
        ink_bar.left_anchor,
        PositionUnit::lerp_fn(style.ink_bar_rect.size.width)
      ) {
        by: transitions::EASE_IN.of(ctx)
      }
    }
  });
  styles.override_compose_style::<CheckBoxStyle>(move |style, host| {
    widget! {
      states { style }
      Ripple {
        center: true,
        color: style.color,
        radius: 20.,
        bounded: RippleBound::Unbounded,
        InteractiveLayer {
          color: style.color, border_radii: Radius::all(20.),
          DynWidget { dyns: host, margin: EdgeInsets::all(12.) }
        }
      }
    }
  });
  styles.override_compose_style::<TabStyle>(move |style, host| {
    widget! {
      track { style }
      Ripple {
        color: style.color,
        InteractiveLayer {
          color: style.color, border_radii: Radius::all(20.),
          DynWidget { dyns: host }
        }
      }
    }
  });
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
  IconTheme::new(icon_size)
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
    families: body_family.clone(),
    ..<_>::default()
  };

  TypographyTheme {
    default_font_family: body_family,
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
