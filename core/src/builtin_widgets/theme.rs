//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use std::{collections::HashMap, rc::Rc};

use ribir_algo::CowArc;
pub use ribir_algo::ShareResource;
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

use crate::{
  declare::DeclareBuilder,
  impl_query_self_only,
  prelude::{Any, BuildCtx, ComposeChild, Declare, Query, QueryFiler, QueryOrder, TypeId, Widget},
  state::State,
};

pub use ribir_painter::*;
pub use ribir_text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

use super::SvgRender;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextSelectedBackground {
  pub focus: Color,
  pub blur: Color,
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
  pub icons: Option<HashMap<NamedSvg, ShareResource<SvgRender>, ahash::RandomState>>,
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
  pub(crate) theme: Rc<Theme>,
}

impl ComposeChild for ThemeWidget {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    use crate::prelude::*;
    widget! {
      init ctx => { ctx.app_ctx().load_font_from_theme(this.theme.clone()); }
      states { this: this.into_readonly() }
      DynWidget {
        dyns: move |ctx: &BuildCtx| {
          ctx.push_theme(this.theme.clone());
          widget_attach_data(child, this.theme.clone())
        }
      }
    }
  }
}

impl Query for Theme {
  impl_query_self_only!();
}

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

    let regular_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato Regular"))]);
    let medium_family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato Regular"))]);

    let typography_theme = typography_theme(
      regular_family,
      medium_family,
      TextDecoration::NONE,
      Color::BLACK.with_alpha(0.87).into(),
    );

    FullTheme {
      palette: Default::default(),
      typography_theme,
      icon_theme: IconTheme::new(icon_size),
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

impl From<FullTheme> for Theme {
  #[inline]
  fn from(value: FullTheme) -> Self { Theme::Full(value) }
}

impl From<InheritTheme> for Theme {
  #[inline]
  fn from(value: InheritTheme) -> Self { Theme::Inherit(value) }
}
