#![feature(test, decl_macro, drain_filter)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;

pub mod animation;
mod application;
pub mod builtin_widgets;
pub(crate) mod composed_widget;
mod context;
pub mod data_widget;
mod stateful;
pub(crate) mod widget_tree;

// todo: reorganize document
#[doc = include_str!("../../docs/declare_macro.md")]
#[doc = include_str!("../../docs/declare_builtin_fields.md")]
pub mod declare;
pub mod dynamic_widget;
pub mod enum_widget;
pub mod events;
pub mod ticker;
pub mod widget;
pub mod widget_children;
pub mod window;
pub mod prelude {
  pub use crate::animation::*;
  #[doc(no_inline)]
  pub use crate::application::Application;
  #[doc(no_inline)]
  pub use crate::builtin_widgets::*;
  #[doc(no_inline)]
  pub use crate::context::*;
  #[doc(no_inline)]
  pub use crate::data_widget::{compose_child_as_data_widget, DataWidget};
  #[doc(no_inline)]
  pub use crate::declare::*;
  #[doc(no_inline)]
  pub use crate::dynamic_widget::*;
  pub use crate::enum_widget::*;
  #[doc(no_inline)]
  pub use crate::events::*;
  #[doc(no_inline)]
  pub use crate::stateful::*;
  #[doc(no_inline)]
  pub use crate::widget;
  #[doc(no_inline)]
  pub use crate::widget::*;
  #[doc(no_inline)]
  pub use crate::widget_children::*;
  #[doc(no_inline)]
  pub use crate::widget_tree::{BoxClamp, WidgetId};
  #[doc(no_inline)]
  pub use crate::window::Window;
  #[doc(no_inline)]
  pub use ::painter::*;
  #[doc(no_inline)]
  pub use algo::CowArc;
  pub use log;
  #[doc(no_inline)]
  pub use ribir_macros::{
    include_svg, widget, widget_try_track, Declare, Lerp, MultiChild, SingleChild, Template,
  };
  #[doc(hidden)]
  pub use rxrust::prelude::*;
}

pub mod test;
