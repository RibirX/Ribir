use ribir_core::prelude::SystemTheme;

pub mod avatar;
pub mod buttons;
pub mod checkbox;
pub mod common_widget;
pub mod divider;
pub mod grid_view;
pub mod icon;
pub mod input;
pub mod label;
pub mod layout;
pub mod link;
pub mod lists;
pub mod path;
pub mod scrollbar;
pub mod tabs;
pub mod text;
pub mod text_field;
pub mod themes;
pub mod transform_box;
pub mod prelude {
  pub use super::avatar::*;
  pub use super::buttons::*;
  pub use super::checkbox::*;
  pub use super::common_widget::*;
  pub use super::divider::*;
  pub use super::grid_view::*;
  pub use super::icon::*;
  pub use super::input::*;
  pub use super::label::*;
  pub use super::layout::*;
  pub use super::link::*;
  pub use super::lists::*;
  pub use super::path::*;
  pub use super::scrollbar::*;
  pub use super::tabs::*;
  pub use super::text::*;
  pub use super::text_field::*;
  pub use super::themes::*;
  pub use super::transform_box::*;
}

pub fn widget_theme_init(theme: &mut SystemTheme) {
  avatar::add_to_system_theme(theme);
  buttons::add_to_system_theme(theme);
  checkbox::add_to_system_theme(theme);
  lists::add_to_system_theme(theme);
  tabs::add_to_system_theme(theme);
  scrollbar::add_to_system_theme(theme);
  input::add_to_system_theme(theme);
  themes::add_to_system_theme(theme);
}
