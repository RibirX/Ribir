use std::rc::Rc;

use ribir_core::{fill_svgs, prelude::*};
use ribir_widgets::prelude::*;
pub mod ripple;
pub mod state_layer;
pub use ripple::*;
pub use state_layer::*;
mod styles_sheet;
pub use styles_sheet::*;

/// Crate a material theme with palette.
fn new(palette: Palette) -> FullTheme {
  let regular_family = Box::new([FontFamily::Name("Roboto".into()), FontFamily::Serif]);
  let medium_family = Box::new([FontFamily::Name("Roboto Medium".into()), FontFamily::Serif]);

  let typography_theme = typography_theme(
    regular_family,
    medium_family,
    TextDecoration::NONE,
    Color::BLACK.with_alpha(0.87).into(),
  );

  let mut theme = FullTheme {
    palette: Rc::new(palette),
    typography_theme,
    icon_theme: icon_theme(),
    transitions_theme: TransitionTheme::default(),
    compose_decorators: <_>::default(),
    custom_styles: <_>::default(),
    font_bytes: Some(vec![
      include_bytes!("./fonts/Roboto-Regular.ttf").to_vec(),
      include_bytes!("./fonts/Roboto-Medium.ttf").to_vec(),
    ]),
    font_files: None,
  };

  fill_svgs! { theme.icon_theme,
    svgs::ADD: "./icons/add_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_BACK: "./icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_DROP_DOWN: "./icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_FORWARD: "./icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CANCEL: "./icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX: "./icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX_OUTLINE_BLANK: "./icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHEVRON_RIGHT: "./icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CLOSE: "./icons/close_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DELETE: "./icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DONE: "./icons/done_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::EXPAND_MORE: "./icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FAVORITE: "./icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::HOME: "./icons/home_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::INDETERMINATE_CHECK_BOX: "./icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGIN: "./icons/login_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGOUT: "./icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MENU: "./icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_VERT: "./icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SEARCH: "./icons/search_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SETTINGS: "./icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::STAR: "./icons/star_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::TEXT_CARET: "./icons/text_caret.svg",
    svgs::MORE_HORIZ: "./icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg"
  };

  fill_svgs! {
    theme.icon_theme,
    material_svgs::ADD_CIRCLE: "./icons/add_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::CHECK_CIRCLE: "./icons/check_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::CHECK: "./icons/check_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::FILE_DOWNLOAD: "./icons/file_download_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::GRADE: "./icons/grade_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::REFRESH: "./icons/refresh_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::SMS: "./icons/sms_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::ACCOUNT_CIRCLE: "./icons/account_circle_FILL0_wght400_GRAD0_opsz48.svg",
    material_svgs::INFO: "./icons/info_FILL0_wght400_GRAD0_opsz48.svg"
  }

  override_compose_decorator(&mut theme);
  init_custom_style(&mut theme);
  theme
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

fn init_custom_style(theme: &mut FullTheme) {
  theme
    .custom_styles
    .set_custom_style(ScrollBarStyle {
      thumb_min_size: 12.,
      thickness: 8.,
      track_brush: theme.palette.primary_container().into(),
    });
  theme
    .custom_styles
    .set_custom_style(CheckBoxStyle {
      icon_size: ICON_SMALL,
      label_style: theme.typography_theme.body_large.text.clone(),
      label_color: theme.palette.on_surface().into(),
    });
  theme
    .custom_styles
    .set_custom_style(InputStyle { size: Some(20.) });
  theme
    .custom_styles
    .set_custom_style(TextAreaStyle { rows: Some(2.), cols: Some(20.) });
  theme
    .custom_styles
    .set_custom_style(SelectedHighLightStyle { brush: Color::from_rgb(181, 215, 254).into() });
  theme
    .custom_styles
    .set_custom_style(FilledButtonStyle {
      height: 40.,
      icon_size: ICON_TINY,
      label_gap: LABEL_GAP,
      icon_pos: IconPosition::Before,
      label_style: theme.typography_theme.label_large.text.clone(),
      radius: BUTTON_RADIUS,
      padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
    });
  theme
    .custom_styles
    .set_custom_style(OutlinedButtonStyle {
      height: 40.,
      icon_size: ICON_TINY,
      label_gap: LABEL_GAP,
      icon_pos: IconPosition::Before,
      label_style: theme.typography_theme.label_large.text.clone(),
      radius: BUTTON_RADIUS,
      padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
      border_width: 1.,
    });
  theme.custom_styles.set_custom_style(ButtonStyle {
    height: 40.,
    icon_size: ICON_TINY,
    label_gap: LABEL_GAP,
    icon_pos: IconPosition::Before,
    label_style: theme.typography_theme.label_large.text.clone(),
    padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
  });
  theme
    .custom_styles
    .set_custom_style(FabButtonStyle {
      height: 56.,
      icon_size: ICON_SMALL,
      label_gap: LABEL_GAP,
      icon_pos: IconPosition::Before,
      label_style: theme.typography_theme.label_large.text.clone(),
      radius: FAB_RADIUS,
      padding_style: EdgeInsets::horizontal(BUTTON_PADDING),
    });
  theme.custom_styles.set_custom_style(TabsStyle {
    extent_with_both: 64.,
    extent_only_label: 48.,
    extent_only_icon: 48.,
    icon_size: theme.icon_theme.icon_size.small,
    icon_pos: Position::Top,
    active_color: theme.palette.primary().into(),
    foreground: theme.palette.on_surface_variant().into(),
    label_style: theme.typography_theme.title_small.text.clone(),
    indicator: IndicatorStyle { extent: 3., measure: Some(INDICATOR_SIZE) },
  });
  theme.custom_styles.set_custom_style(AvatarStyle {
    size: Size::splat(AVATAR_SIZE),
    radius: Some(AVATAR_RADIUS),
    text_style: theme.typography_theme.body_large.text.clone(),
  });
  theme
    .custom_styles
    .set_custom_style(ListItemStyle {
      padding_style: Some(EdgeInsets { left: 0., right: 24., bottom: 8., top: 8. }),
      item_align: |num| {
        if num >= 2 { Align::Start } else { Align::Center }
      },
      label_gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
      headline_style: theme.typography_theme.body_large.text.clone(),
      supporting_style: theme.typography_theme.body_medium.text.clone(),
      leading_config: EdgeWidgetStyle {
        icon: EdgeItemStyle { size: ICON_SMALL, gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)) },
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
        poster: EdgeItemStyle { size: Size::new(120., 64.), gap: None },
        custom: EdgeItemStyle {
          size: Size::splat(LIST_ITEM_SIZE),
          gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
        },
      },
      trailing_config: EdgeWidgetStyle {
        icon: EdgeItemStyle { size: ICON_SMALL, gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)) },
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
        poster: EdgeItemStyle { size: Size::new(120., 64.), gap: None },
        custom: EdgeItemStyle {
          size: Size::splat(LIST_ITEM_SIZE),
          gap: Some(EdgeInsets::only_left(LIST_ITEM_GAP)),
        },
      },
    });
  theme
    .custom_styles
    .set_custom_style(PlaceholderStyle {
      foreground: theme.palette.on_surface_variant().into(),
      text_style: theme.typography_theme.body_medium.text.clone(),
    });
}

fn override_compose_decorator(theme: &mut FullTheme) {
  fn scrollbar_thumb(host: Widget, margin: EdgeInsets) -> impl WidgetBuilder {
    fn_widget! {
      @$host {
        margin,
        border_radius: Radius::all(4.),
        background: Palette::of(ctx!()).primary(),
      }
    }
  }

  let styles = &mut theme.compose_decorators;
  styles.override_compose_decorator::<HScrollBarThumbDecorator>(|this, host, ctx| {
    fn_widget! {
      let host = scrollbar_thumb(host, EdgeInsets::vertical(1.));
      let mut thumb = @ $host { anchor: pipe!($this.offset).map(Anchor::left) };
      thumb
        .get_relative_anchor_widget()
        .map_writer(|w| PartData::from_ref_mut(&mut w.anchor))
        .transition(transitions::LINEAR.of(ctx!()), ctx!());
      thumb
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<VScrollBarThumbDecorator>(|this, host, ctx| {
    fn_widget! {
      let host = scrollbar_thumb(host, EdgeInsets::vertical(1.));
      let mut thumb = @ $host { anchor: pipe!($this.offset).map(Anchor::top) };
      thumb
        .get_relative_anchor_widget()
        .map_writer(|w| PartData::from_ref_mut(&mut w.anchor))
        .transition(transitions::LINEAR.of(ctx!()), ctx!());

      thumb
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<IndicatorDecorator>(|style, host, ctx| {
    fn_widget! {
      let mut indicator = @ $host {
        anchor: pipe!{
          let style = $style;
          let x = match style.pos {
            Position::Top | Position::Bottom => style.rect.origin.x
              + (style.rect.size.width - INDICATOR_SIZE) / 2.,
            Position::Left => style.rect.size.width - style.extent,
            Position::Right => 0.,
          };
          let y = match style.pos {
            Position::Left | Position::Right => style.rect.origin.y
              + (style.rect.size.height - INDICATOR_SIZE) / 2.,
            Position::Top => style.rect.size.height - style.extent,
            Position::Bottom => 0.,
          };
          Anchor::left_top(x, y)
        },
      };

      indicator
        .get_relative_anchor_widget()
        .map_writer(|w| PartData::from_ref_mut(&mut w.anchor))
        .transition(transitions::EASE_IN.of(ctx!()), ctx!());
      indicator
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<CheckBoxDecorator>(move |style, host, ctx| {
    fn_widget! {
      @Ripple {
        center: true,
        color: pipe!($style.color),
        radius: 24.,
        bounded: RippleBound::Unbounded,
        @InteractiveLayer {
          color: pipe!($style.color),
          border_radii: Radius::all(24.),
          @$host {
            margin: EdgeInsets::all(12.)
          }
        }
      }
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<FilledButtonDecorator>(move |style, host, ctx| {
    fn_widget! {
      @Ripple {
        center: false,
        color: {
          let palette = Palette::of(ctx!()).clone();
          pipe!(palette.on_of(&palette.base_of(&$style.color)))
        },
        bounded: RippleBound::Radius(Radius::all(20.)),
        @InteractiveLayer {
          border_radii: Radius::all(20.),
          color: {
            let palette = Palette::of(ctx!()).clone();
            pipe!(palette.on_of(&palette.base_of(&$style.color)))
          },
          @$host {
            margin: EdgeInsets::all(0.)
          }
        }
      }
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<OutlinedButtonDecorator>(move |style, host, ctx| {
    fn_widget! {
      @Ripple {
        center: false,
        color: {
          let palette = Palette::of(ctx!()).clone();
          pipe!(palette.base_of(&$style.color))
        },
        bounded: RippleBound::Radius(Radius::all(20.)),
        @InteractiveLayer {
          border_radii: Radius::all(20.),
          color: {
            let palette = Palette::of(ctx!()).clone();
            pipe!(palette.base_of(&$style.color))
          },
          @$host {
            margin: EdgeInsets::all(0.)
          }
        }
      }
    }
    .build(ctx)
  });
  styles.override_compose_decorator::<ButtonDecorator>(move |style, host, ctx| {
    fn_widget! {
      @Ripple {
        center: false,
        color: {
          let palette = Palette::of(ctx!()).clone();
          pipe!(palette.on_of(&palette.base_of(&$style.color)))
        },
        bounded: RippleBound::Radius(Radius::all(20.)),
        @InteractiveLayer {
          border_radii: Radius::all(20.),
          color: {
            let palette = Palette::of(ctx!()).clone();
            pipe!(palette.on_of(&palette.base_of(&$style.color)))
          },
          @$host {
            margin: EdgeInsets::all(0.)
          }
        }
      }
    }
    .build(ctx)
  });
  let textfield = TextFieldThemeSuit::from_theme(&theme.palette, &theme.typography_theme);
  theme.custom_styles.set_custom_style(textfield);
}

pub mod purple {
  use super::*;

  fn palette(brightness: Brightness) -> Palette {
    Palette {
      primary: Color::from_u32(0x6750A4FF),
      secondary: Color::from_u32(0x625B71FF),
      tertiary: Color::from_u32(0x7D5260FF),
      neutral: Color::from_u32(0xFFFBFEFF),
      neutral_variant: Color::from_u32(0xE7E0ECFF),
      error: Color::from_u32(0xB3261EFF),
      warning: Color::from_u32(0xFFB74DFF),
      success: Color::from_u32(0x81C784FF),
      brightness,
      light: LightnessCfg::light_theme_default(),
      dark: LightnessCfg::dark_theme_default(),
    }
  }

  /// A default light blue theme. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn light() -> FullTheme { new(palette(Brightness::Light)) }

  /// A default dark theme with a teal accent color. Colors from <https://material.io/design/color/dark-theme.html#ui-application>
  pub fn dark() -> FullTheme { new(palette(Brightness::Dark)) }
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
  regular_family: Box<[FontFamily]>, medium_family: Box<[FontFamily]>, decoration: TextDecoration,
  decoration_color: Brush,
) -> TypographyTheme {
  let decoration = TextDecorationStyle { decoration, decoration_color };
  let regular_face =
    FontFace { families: regular_family.clone(), weight: FontWeight::NORMAL, ..<_>::default() };
  let medium_face =
    FontFace { families: medium_family, weight: FontWeight::MEDIUM, ..<_>::default() };

  TypographyTheme {
    display_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(64.0.into())),
        font_size: FontSize::Pixel(57.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    display_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(52.0.into())),
        font_size: FontSize::Pixel(45.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    display_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(44.0.into())),
        font_size: FontSize::Pixel(36.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(40.0.into())),
        font_size: FontSize::Pixel(32.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(36.0.into())),
        font_size: FontSize::Pixel(28.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(32.0.into())),
        font_size: FontSize::Pixel(24.0.into()),
        letter_space: Some(0.0.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(28.0.into())),
        font_size: FontSize::Pixel(22.0.into()),
        letter_space: Some(0.0.into()),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(24.0.into())),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(0.15.into()),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(20.0.into())),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(0.1.into()),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(20.0.into())),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(0.1.into()),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(16.0.into())),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(0.5.into()),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(16.0.into())),
        font_size: FontSize::Pixel(11.0.into()),
        letter_space: Some(0.5.into()),
        font_face: medium_face,
      }),
      decoration: decoration.clone(),
    },
    body_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(24.0.into())),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(0.5.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(20.0.into())),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(0.25.into()),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Em::from_pixel(16.0.into())),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(0.4.into()),
        font_face: regular_face,
      }),
      decoration,
    },
  }
}
