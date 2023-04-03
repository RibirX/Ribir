use std::rc::Rc;

pub use super::*;
use crate::prelude::*;
use ribir_core::{fill_svgs, prelude::*};
pub mod ripple;
pub mod state_layer;
pub use ripple::*;
pub use state_layer::*;

/// Crate a material theme with palette.
pub fn new(brightness: Brightness, palette: Palette) -> Theme {
  let regular_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed(
    "Roboto Regular",
  ))]);
  let medium_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed(
    "Roboto Medium",
  ))]);

  let typography_theme = typography_theme(
    regular_family,
    medium_family,
    TextDecoration::NONE,
    Color::BLACK.with_alpha(0.87).into(),
  );

  let mut theme = FullTheme {
    brightness,
    palette: Rc::new(palette),
    typography_theme,
    icon_theme: icon_theme(),
    transitions_theme: TransitionTheme::default(),
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
    svgs::STAR: "./material/icons/star_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::TEXT_CARET: "./material/icons/text_caret.svg",
    svgs::SMS: "./material/icons/sms_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ACCOUNT_CIRCLE: "./material/icons/account_circle_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_HORIZ: "./material/icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg"
  };

  override_compose_style(&mut theme);
  init_custom_theme(&mut theme);
  Theme::Full(theme)
}

const FAB_RADIUS: f32 = 16.;
const LABEL_GAP: f32 = 8.;
const BUTTON_RADIUS: f32 = 20.;
const BUTTON_PADDING: f32 = 16.;
const INDICATOR_SIZE: f32 = 60.;
const LIST_ITEM_GAP: f32 = 16.;
const LIST_ITEM_SIZE: f32 = 40.;
const AVATAR_SIZE: f32 = 40.;
const AVATAR_RADIUS: f32 = 20.;
const LIST_IMAGE_ITEM_SIZE: f32 = 56.;

const ICON_TINY: Size = Size::new(18., 18.);
const ICON_SMALL: Size = Size::new(24., 24.);
const ICON_MEDIUM: Size = Size::new(36., 36.);
const ICON_LARGE: Size = Size::new(48., 48.);
const ICON_HUGE: Size = Size::new(64., 64.);

fn init_custom_theme(theme: &mut FullTheme) {
  theme.custom_themes.set_custom_theme(ScrollBarTheme {
    thumb_min_size: 12.,
    thickness: 8.,
    track_brush: theme.palette.primary_container().into(),
  });
  theme.custom_themes.set_custom_theme(CheckBoxStyle {
    icon_size: ICON_TINY,
    label_style: theme.typography_theme.body_large.text.clone(),
    label_foreground: theme.palette.on_surface().into(),
    position: Position::Right,
  });
  theme.custom_themes.set_custom_theme(InputTheme {
    min_length: 20.,
    select_background: Color::from_rgb(181, 215, 254).into(),
    caret_color: Brush::Color(theme.palette.on_surface()),
  });
  theme.custom_themes.set_custom_theme(FilledButtonStyle {
    height: 40.,
    icon_size: ICON_TINY,
    label_gap: LABEL_GAP,
    icon_pos: IconPosition::Before,
    label_style: theme.typography_theme.label_large.text.clone(),
    radius: BUTTON_RADIUS,
    padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
  });
  theme.custom_themes.set_custom_theme(OutlinedButtonStyle {
    height: 40.,
    icon_size: ICON_TINY,
    label_gap: LABEL_GAP,
    icon_pos: IconPosition::Before,
    label_style: theme.typography_theme.label_large.text.clone(),
    radius: BUTTON_RADIUS,
    padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
    border_width: 1.,
  });
  theme.custom_themes.set_custom_theme(RawButtonStyle {
    height: 40.,
    icon_size: ICON_TINY,
    label_gap: LABEL_GAP,
    icon_pos: IconPosition::Before,
    label_style: theme.typography_theme.label_large.text.clone(),
    padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
  });
  theme.custom_themes.set_custom_theme(FabButtonStyle {
    height: 56.,
    icon_size: ICON_SMALL,
    label_gap: LABEL_GAP,
    icon_pos: IconPosition::Before,
    label_style: theme.typography_theme.label_large.text.clone(),
    radius: FAB_RADIUS,
    padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
  });
  theme.custom_themes.set_custom_theme(TabsStyle {
    extent_with_both: 64.,
    extent_only_label: 48.,
    extent_only_icon: 48.,
    icon_size: theme.icon_theme.icon_size.small,
    icon_pos: Position::Top,
    active_color: theme.palette.primary().into(),
    foreground: theme.palette.on_surface_variant().into(),
    label_style: theme.typography_theme.title_small.text.clone(),
    indicator: IndicatorStyle {
      extent: 3.,
      measure: Some(INDICATOR_SIZE),
    },
  });
  theme.custom_themes.set_custom_theme(ListsStyle {
    padding: EdgeInsets::vertical(8.),
    background: theme.palette.surface().into(),
  });
  theme.custom_themes.set_custom_theme(AvatarStyle {
    size: Size::splat(AVATAR_SIZE),
    radius: Some(AVATAR_RADIUS),
    background: Some(theme.palette.primary().into()),
    text_color: theme.palette.on_primary().into(),
    text_style: theme.typography_theme.body_large.text.clone(),
  });
  theme.custom_themes.set_custom_theme(ListItemStyle {
    padding_style: Some(EdgeInsets {
      left: 0.,
      right: 24.,
      bottom: 8.,
      top: 8.,
    }),
    item_align: |num| {
      if num >= 2 {
        Align::Start
      } else {
        Align::Center
      }
    },
    label_gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
    headline_style: theme.typography_theme.body_large.text.clone(),
    supporting_style: theme.typography_theme.body_medium.text.clone(),
    leading_config: EdgeWidgetStyle {
      icon: EdgeItemStyle {
        size: ICON_SMALL,
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      text: EdgeTextItemStyle {
        style: theme.typography_theme.label_small.text.clone(),
        foreground: theme.palette.on_surface_variant().into(),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      avatar: EdgeItemStyle {
        size: Size::splat(LIST_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      image: EdgeItemStyle {
        size: Size::splat(LIST_IMAGE_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      poster: EdgeItemStyle {
        size: Size::new(120., 64.),
        gap: None,
      },
      custom: EdgeItemStyle {
        size: Size::splat(LIST_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
    },
    trailing_config: EdgeWidgetStyle {
      icon: EdgeItemStyle {
        size: ICON_SMALL,
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      text: EdgeTextItemStyle {
        style: theme.typography_theme.label_small.text.clone(),
        foreground: theme.palette.on_surface_variant().into(),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      avatar: EdgeItemStyle {
        size: Size::splat(LIST_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      image: EdgeItemStyle {
        size: Size::splat(LIST_IMAGE_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
      poster: EdgeItemStyle {
        size: Size::new(120., 64.),
        gap: None,
      },
      custom: EdgeItemStyle {
        size: Size::splat(LIST_ITEM_SIZE),
        gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      },
    },
  });
}

fn override_compose_style(theme: &mut FullTheme) {
  fn scrollbar_thumb(host: Widget, margin: EdgeInsets) -> Widget {
    widget! {
      init ctx => {
        let background = Palette::of(ctx).primary();
      }
      DynWidget {
        dyns: host,
        margin,
        border_radius: Radius::all(4.),
        background
      }
    }
  }

  let styles = &mut theme.compose_styles;
  styles.override_compose_style::<HScrollBarThumbStyle>(|this, host| {
    widget! {
      states { this }
      init ctx => {
        let  smooth_scroll = transitions::SMOOTH_SCROLL.of(ctx);
      }
      DynWidget {
        id: thumb,
        left_anchor: this.offset,
        dyns: scrollbar_thumb(host, EdgeInsets::vertical(1.))
      }

      transition prop!(thumb.left_anchor, PositionUnit::lerp_fn(thumb.layout_width())) {
        by: smooth_scroll,
      }
    }
  });
  styles.override_compose_style::<VScrollBarThumbStyle>(|this, host| {
    widget! {
      states { this }
      init ctx => {
        let smooth_scroll = transitions::SMOOTH_SCROLL.of(ctx);
      }
      DynWidget {
        id: thumb,
        top_anchor: this.offset,
        dyns: scrollbar_thumb(host, EdgeInsets::vertical(1.))
      }

      transition prop!(thumb.top_anchor, PositionUnit::lerp_fn(thumb.layout_height())) {
        by: smooth_scroll
      }
    }
  });
  styles.override_compose_style::<IndicatorDecorator>(|style, host| {
    widget! {
      states { style }
      init ctx => {
        let ease_in = transitions::EASE_IN.of(ctx);
      }
      DynWidget {
        id: indicator,
        left_anchor: match style.pos {
          Position::Top | Position::Bottom => style.rect.origin.x
            + (style.rect.size.width - INDICATOR_SIZE) / 2.,
          Position::Left => style.rect.size.width - style.extent,
          Position::Right => 0.,
        },
        top_anchor: match style.pos {
          Position::Left | Position::Right => style.rect.origin.y
            + (style.rect.size.height - INDICATOR_SIZE) / 2.,
          Position::Top => style.rect.size.height - style.extent,
          Position::Bottom => 0.,
        },
        dyns: host,
      }
      transition prop!(
        indicator.left_anchor,
        PositionUnit::lerp_fn(style.rect.size.width)
      ) { by: ease_in.clone() }
      transition prop!(
        indicator.top_anchor,
        PositionUnit::lerp_fn(style.rect.size.height)
      ) { by: ease_in }
    }
  });
  styles.override_compose_style::<CheckBoxDecorator>(move |style, host| {
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
  let textfield = TextFieldThemeSuit::from_theme(&theme.palette, &theme.typography_theme);
  theme.custom_themes.set_custom_theme(textfield);
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
      warning: Color::from_u32(0xFFB74DFF),
      success: Color::from_u32(0x81C784FF),
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
    tiny: ICON_TINY,
    small: ICON_SMALL,
    medium: ICON_MEDIUM,
    large: ICON_LARGE,
    huge: ICON_HUGE,
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
  regular_family: Box<[FontFamily]>,
  medium_family: Box<[FontFamily]>,
  decoration: TextDecoration,
  decoration_color: Brush,
) -> TypographyTheme {
  let decoration = TextDecorationStyle { decoration, decoration_color };
  let regular_face = FontFace {
    families: regular_family.clone(),
    weight: FontWeight::NORMAL,
    ..<_>::default()
  };
  let medium_face = FontFace {
    families: medium_family,
    weight: FontWeight::MEDIUM,
    ..<_>::default()
  };

  TypographyTheme {
    default_font_family: regular_family,
    display_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(64.0.into()).into()),
        font_size: FontSize::Pixel(57.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    display_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(52.0.into()).into()),
        font_size: FontSize::Pixel(45.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    display_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(44.0.into()).into()),
        font_size: FontSize::Pixel(36.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    headline_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(40.0.into()).into()),
        font_size: FontSize::Pixel(32.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    headline_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(36.0.into()).into()),
        font_size: FontSize::Pixel(28.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    headline_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(32.0.into()).into()),
        font_size: FontSize::Pixel(24.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    title_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(28.0.into()).into()),
        font_size: FontSize::Pixel(22.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: medium_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    title_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.15.into())),
        font_face: medium_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    title_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    label_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    label_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    label_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(11.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face,
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    body_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    body_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.25.into())),
        font_face: regular_face.clone(),
        path_style: PathStyle::Fill,
      }),
      decoration: decoration.clone(),
    },
    body_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.4.into())),
        font_face: regular_face,
        path_style: PathStyle::Fill,
      }),
      decoration,
    },
  }
}
