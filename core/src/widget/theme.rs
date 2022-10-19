//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used as an attribute to attach to a widget, query
//! theme data from `BuildCtx`. Use `Theme` widgets to specify part of
//! application's theme. Application theme is use `Theme` widget as root of all
//! windows.
pub mod material;
mod palette;

use std::rc::Rc;

pub use palette::Palette;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
use ribir_macros::widget_try_track;
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;
mod compose_styles;
pub use compose_styles::*;
mod custom_theme;
pub use custom_theme::*;

use crate::{
  impl_query_self_only,
  prelude::{Any, BuildCtx, Declare, Query, QueryFiler, QueryOrder, TypeId, Widget},
};
use algo::ShareResource;
pub use painter::*;
pub use text::{FontFace, FontFamily, FontSize, FontWeight, Pixel};

use super::{
  data_widget::{expr_attach_data, widget_attach_data},
  ComposeChild, StateWidget,
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
