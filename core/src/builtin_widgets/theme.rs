//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use crate::{
  declare::DeclareBuilder,
  fill_svgs, impl_query_self_only,
  prelude::include_svg,
  prelude::{Any, BuildCtx, ComposeChild, Declare, Query, QueryFiler, QueryOrder, TypeId},
  state::State,
  widget::{Widget, WidgetBuilder},
};
use ribir_algo::CowArc;
pub use ribir_algo::ShareResource;
use ribir_geom::Size;
use ribir_text::TextStyle;
use std::{collections::HashMap, rc::Rc};

mod palette;
pub use palette::*;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;
mod compose_decorators;
pub use compose_decorators::*;
mod custom_styles;
pub use custom_styles::*;

pub use ribir_painter::*;
pub use ribir_text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}

/// A full theme means all config have be defined in it. Everything of parent
/// theme are overriding here, if anything that you can't find means it be
/// override as undefine, should not continue find in parent theme.
pub struct FullTheme {
  pub palette: Rc<Palette>,
  pub typography_theme: TypographyTheme,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  pub compose_decorators: ComposeDecorators,
  pub custom_styles: CustomStyles,
  pub font_bytes: Option<Vec<Vec<u8>>>,
  pub font_files: Option<Vec<String>>,
}

/// Inherit theme override part of parent theme, if anything not found in here,
/// should query in parent theme until meet a `FullTheme`.
#[derive(Default)]
pub struct InheritTheme {
  pub palette: Option<Rc<Palette>>,
  pub typography_theme: Option<TypographyTheme>,
  /// icon size standard
  pub icon_size: Option<IconSize>,
  /// a collection of icons.
  pub icons: Option<HashMap<NamedSvg, ShareResource<Svg>, ahash::RandomState>>,
  pub transitions_theme: Option<TransitionTheme>,
  pub compose_decorators: Option<ComposeDecorators>,
  pub custom_styles: Option<CustomStyles>,
  pub font_bytes: Option<Vec<Vec<u8>>>,
  pub font_files: Option<Vec<String>>,
}

pub enum Theme {
  Full(FullTheme),
  Inherit(InheritTheme),
}

#[derive(Declare)]
pub struct ThemeWidget {
  pub theme: Rc<Theme>,
}

impl ComposeChild for ThemeWidget {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    use crate::prelude::*;
    FnWidget::new(move |ctx| {
      let theme = match this {
        State::Stateless(t) => t.theme,
        State::Stateful(s) => s.state_ref().theme.clone(),
      };

      AppCtx::load_font_from_theme(&theme);
      ctx.push_theme(theme.clone());
      let p = DataWidget::new(Box::new(Void), theme).build(ctx);
      let old = ctx.force_as_mut().reset_ctx_from(Some(p));
      let c = child.build(ctx);
      ctx.append_child(p, c);
      ctx.force_as_mut().reset_ctx_from(old);
      ctx.pop_theme();
      p
    })
    .into()
  }
}

impl_query_self_only!(Theme);

impl Default for Theme {
  fn default() -> Self { Theme::Full(<_>::default()) }
}
impl Default for FullTheme {
  fn default() -> Self {
    let icon_size = IconSize {
      tiny: Size::new(18., 18.),
      small: Size::new(24., 24.),
      medium: Size::new(36., 36.),
      large: Size::new(48., 48.),
      huge: Size::new(64., 64.),
    };

    let regular_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);
    let medium_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato"))]);

    let typography_theme = typography_theme(
      regular_family,
      medium_family,
      TextDecoration::NONE,
      Color::BLACK.with_alpha(0.87).into(),
    );

    let mut icon_theme = IconTheme::new(icon_size);
    fill_svgs! {
      icon_theme,
      svgs::ADD: "./theme/icons/add_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_BACK: "./theme/icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_DROP_DOWN: "./theme/icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::ARROW_FORWARD: "./theme/icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CANCEL: "./theme/icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHECK_BOX: "./theme/icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHECK_BOX_OUTLINE_BLANK: "./theme/icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CHEVRON_RIGHT: "./theme/icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::CLOSE: "./theme/icons/close_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::DELETE: "./theme/icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::DONE: "./theme/icons/done_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::EXPAND_MORE: "./theme/icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::FAVORITE: "./theme/icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::HOME: "./theme/icons/home_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::INDETERMINATE_CHECK_BOX: "./theme/icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::LOGIN: "./theme/icons/login_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::LOGOUT: "./theme/icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MENU: "./theme/icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MORE_HORIZ: "./theme/icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::MORE_VERT: "./theme/icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::OPEN_IN_NEW: "./theme/icons/open_in_new_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::SEARCH: "./theme/icons/search_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::SETTINGS: "./theme/icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::STAR: "./theme/icons/star_FILL0_wght400_GRAD0_opsz48.svg",
      svgs::TEXT_CARET: "./theme/icons/text_caret.svg"
    };

    FullTheme {
      palette: Default::default(),
      typography_theme,
      icon_theme,
      transitions_theme: Default::default(),
      compose_decorators: Default::default(),
      custom_styles: Default::default(),
      font_bytes: None,
      font_files: None,
    }
  }
}

fn typography_theme(
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
      }),
      decoration: decoration.clone(),
    },
    display_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(52.0.into()).into()),
        font_size: FontSize::Pixel(45.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    display_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(44.0.into()).into()),
        font_size: FontSize::Pixel(36.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(40.0.into()).into()),
        font_size: FontSize::Pixel(32.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(36.0.into()).into()),
        font_size: FontSize::Pixel(28.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    headline_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(32.0.into()).into()),
        font_size: FontSize::Pixel(24.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(28.0.into()).into()),
        font_size: FontSize::Pixel(22.0.into()),
        letter_space: Some(Pixel(0.0.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.15.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    title_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.1.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    label_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(11.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: medium_face,
      }),
      decoration: decoration.clone(),
    },
    body_large: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(24.0.into()).into()),
        font_size: FontSize::Pixel(16.0.into()),
        letter_space: Some(Pixel(0.5.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_medium: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(20.0.into()).into()),
        font_size: FontSize::Pixel(14.0.into()),
        letter_space: Some(Pixel(0.25.into())),
        font_face: regular_face.clone(),
      }),
      decoration: decoration.clone(),
    },
    body_small: TextTheme {
      text: CowArc::owned(TextStyle {
        line_height: Some(Pixel(16.0.into()).into()),
        font_size: FontSize::Pixel(12.0.into()),
        letter_space: Some(Pixel(0.4.into())),
        font_face: regular_face,
      }),
      decoration,
    },
  }
}

impl From<FullTheme> for Theme {
  #[inline]
  fn from(value: FullTheme) -> Self { Theme::Full(value) }
}

impl From<InheritTheme> for Theme {
  #[inline]
  fn from(value: InheritTheme) -> Self { Theme::Inherit(value) }
}
