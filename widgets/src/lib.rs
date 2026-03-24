#[macro_use]
extern crate bitflags;

use ribir_core::prelude::Provider;

pub mod animation;
pub mod avatar;
pub mod badge;
pub mod buttons;
pub mod checkbox;
pub mod common_widget;
pub mod divider;
pub mod grid_view;
pub mod icon;
pub mod input;
pub mod label;
pub mod layout;
pub mod list;
pub mod menu;
pub mod navigation_rail;
pub mod overlay;
pub mod path;
pub mod progress;
pub mod radio;
pub mod router;
pub mod scrollbar;
pub mod select_region;
pub mod slider;
pub mod switch;
pub mod tabs;
pub mod tooltip;

pub mod transform_box;

/// Returns the default providers for widgets.
///
/// This function provides all necessary providers for widgets to function
/// correctly, including tooltip support.
pub fn default_providers() -> [Provider; 1] { [tooltip::default_tooltip_provider()] }

#[doc(hidden)]
pub use ribir_core as core;

pub mod prelude {
  pub use super::{
    animation::*, avatar::*, badge::*, buttons::*, checkbox::*, common_widget::*, divider::*,
    grid_view::*, icon::*, input::*, label::*, layout::*, list::*, menu::*, navigation_rail::*,
    overlay::*, path::*, progress::*, radio::*, router::*, scrollbar::*, select_region::*,
    slider::*, switch::*, tabs::*, tooltip::*, transform_box::*,
  };
  pub use crate::{cases, transitions};
}
