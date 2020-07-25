//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used when create widget, query theme data from
//! `BuildCtx`. Use `Theme` widgets to specify part of application's theme.
//! Application theme is use `Theme` widget as root of all windows.
pub mod material;
pub mod theme_data;
use crate::prelude::*;
pub use theme_data::ThemeData;

#[derive(Debug)]
pub struct Theme {
  pub data: ThemeData,
  pub widget: BoxWidget,
}

inherit_widget!(Theme, widget);
