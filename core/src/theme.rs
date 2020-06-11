//! To share colors and font styles throughout an app or sub widget tree, use
//! themes. Theme data can be used when create widget, query theme data from
//! `BuildCtx`. Use `Theme` widgets to specify part of application's theme.
//! Application theme is use `Theme` widget as root of all windows.

use canvas::Color;

#[derive(Clone)]
pub struct ThemeData {
  // This is a dark or light theme.
  pub brightness: DarkMode,
  // The background color for major parts of the app
  pub primary_color: Color,
  // The foreground color for widgets
  pub accent_color: Color,
  // Default text font family
  pub default_font_family: String,
}

pub enum DarkMode {
  Black,
  Light,
}
