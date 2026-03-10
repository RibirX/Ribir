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
pub mod path;
pub mod progress;
pub mod radio;
pub mod router;
pub mod scrollbar;
pub mod select_region;
pub mod slider;
pub mod switch;
pub mod tabs;

pub mod transform_box;

#[doc(hidden)]
pub use ribir_core as core;

pub mod prelude {
  pub use super::{
    animation::*, avatar::*, badge::*, buttons::*, checkbox::*, common_widget::*, divider::*,
    grid_view::*, icon::*, input::*, label::*, layout::*, list::*, menu::*, path::*, progress::*,
    radio::*, router::*, scrollbar::*, select_region::*, slider::*, switch::*, tabs::*,
    transform_box::*,
  };
  pub use crate::{cases, transitions};
}
