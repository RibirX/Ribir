//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.
pub mod material;
mod palette;

pub use palette::Palette;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;

use crate::{
  impl_proxy_query, impl_query_self_only,
  prelude::{Any, BuildCtx, Declare, Query, QueryFiler, QueryOrder, TypeId, Widget},
};
use algo::ShareResource;
pub use painter::*;
pub use text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

use super::{
  data_widget::compose_child_as_data_widget, ComposeChild,
  StateWidget, 
};

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

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBoxDecorationStyle {
  pub background: Brush,
  /// The corners of this box are rounded by this `BorderRadius`. The round
  /// corner only work if the two borders beside it are same style.]
  pub radius: Option<Radius>,
  /// The thickness of scrollbar element.
  pub thickness: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollBarTheme {
  pub track: ScrollBoxDecorationStyle,
  pub thumb: ScrollBoxDecorationStyle,
  /// The min size of the thumb have.
  pub thumb_min_size: f32,
}

pub struct Theme {
  // Dark or light theme.
  pub brightness: Brightness,
  pub palette: Palette,
  pub typography_theme: TypographyTheme,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  /// Default text font families
  pub default_font_family: Box<[FontFamily]>,
  pub scrollbar: ScrollBarTheme,
  pub text_selected_background: TextSelectedBackground,
  pub caret_color: Color,
  // compose_styles map.
  // custom config by type id.
}

impl TextSelectedBackground {
  #[inline]
  pub fn of<'a>(ctx: &'a mut BuildCtx) -> &'a Self { &&ctx.theme().text_selected_background }
}

impl ScrollBarTheme {
  #[inline]
  pub fn of<'a>(ctx: &'a mut BuildCtx) -> &'a Self { &ctx.theme().scrollbar }
}

#[derive(Declare)]
pub struct ThemeWidget {
  #[declare(builtin)]
  pub theme: Theme,
}

impl ComposeChild for ThemeWidget {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    // todo: theme can provide fonts to load.
    compose_child_as_data_widget(child, this)
  }
}

impl Query for Theme {
  impl_query_self_only!();
}

impl Query for ThemeWidget {
  impl_proxy_query!(theme);
}
