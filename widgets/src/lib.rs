#[macro_use]
extern crate bitflags;

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

#[doc(hidden)]
pub use ribir_core as core;

/// Re-export Follow from core since it requires internal APIs
pub use crate::core::builtin_widgets::Follow;

pub mod prelude {
  pub use super::{
    Follow, animation::*, avatar::*, badge::*, buttons::*, checkbox::*, common_widget::*,
    divider::*, grid_view::*, icon::*, input::*, label::*, layout::*, list::*, menu::*, overlay::*,
    path::*, progress::*, radio::*, router::*, scrollbar::*, select_region::*, slider::*,
    switch::*, tabs::*, tooltip::*, transform_box::*,
  };
  pub use crate::{cases, transitions};
}
