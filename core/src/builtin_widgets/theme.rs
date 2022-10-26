//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.

use std::rc::Rc;

pub use algo::ShareResource;
mod palette;
pub use palette::*;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
use ribir_macros::{include_svg, widget_try_track};
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;
mod compose_styles;
pub use compose_styles::*;
mod custom_theme;
pub use custom_theme::*;

use crate::{
  impl_query_self_only,
  prelude::{Any, BuildCtx, ComposeChild, Declare, Query, QueryFiler, QueryOrder, TypeId, Widget},
  widget::StateWidget,
};

pub use painter::*;
pub use text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

use crate::data_widget::{expr_attach_data, widget_attach_data};

use super::SvgRender;

#[derive(Clone, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextSelectedBackground {
  pub focus: Color,
  pub blur: Color,
}

#[derive(Clone)]
pub struct Theme {
  // Dark or light theme.
  pub brightness: Brightness,
  pub palette: Palette,
  pub typography_theme: TypographyTheme,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  pub compose_styles: ComposeStyles,
  pub custom_themes: CustomThemes,

  // todo: refactor input theme style.
  pub text_selected_background: TextSelectedBackground,
  pub caret_color: Color,
}

impl TextSelectedBackground {
  #[inline]
  pub fn of<'a>(ctx: &'a BuildCtx) -> &'a Self { &&ctx.theme().text_selected_background }
}

#[derive(Declare)]
pub struct ThemeWidget {
  #[declare(builtin)]
  pub theme: Rc<Theme>,
}

impl ComposeChild for ThemeWidget {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    use crate::prelude::*;

    widget_try_track! {
      try_track { this }
      // use `ExprWidget` to refresh whole subtree when theme changed.
      ExprWidget {
        expr: move |ctx: &mut BuildCtx| {
          let theme = this.theme.clone();
          ctx.theme = theme.clone();
          widget_attach_data(child, theme, expr_attach_data)
        }
      }
    }
  }
}

impl Query for Theme {
  impl_query_self_only!();
}

impl Default for Theme {
  fn default() -> Self {
    let palette = Palette::default();
    let icon_size = IconSize {
      tiny: Size::new(18., 18.),
      small: Size::new(24., 24.),
      medium: Size::new(36., 36.),
      large: Size::new(48., 48.),
      huge: Size::new(64., 64.),
    };
    let miss_icon = ShareResource::new(SvgRender(include_svg!("./theme/miss_icon.svg")));

    let text_selected_background = TextSelectedBackground {
      focus: palette.primary_container(),
      blur: palette.surface_variant(),
    };

    let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))]);
    let typography_theme = typography_theme(
      family.clone(),
      family.clone(),
      palette.on_background().into(),
      palette.on_surface_variant().into(),
      TextDecoration::NONE,
      Color::BLACK.with_alpha(0.87).into(),
    );

    Self {
      brightness: Brightness::Light,
      palette: Default::default(),
      typography_theme,
      icon_theme: IconTheme::new(icon_size, miss_icon),
      transitions_theme: Default::default(),
      compose_styles: Default::default(),
      custom_themes: Default::default(),
      text_selected_background,
      caret_color: Default::default(),
    }
  }
}

fn typography_theme(
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
