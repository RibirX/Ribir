#![feature(test, decl_macro)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;

pub mod animation;
mod application;
pub mod builtin_widgets;
pub(crate) mod composed_widget;
mod context;
pub mod data_widget;
mod state;
pub(crate) mod widget_tree;

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
  pub use crate::data_widget::{
    compose_child_as_data_widget, widget_attach_data, AnonymousData, DataWidget,
  };
  #[doc(no_inline)]
  pub use crate::declare::*;
  #[doc(no_inline)]
  pub use crate::dynamic_widget::*;
  pub use crate::enum_widget::*;
  #[doc(no_inline)]
  pub use crate::events::*;
  #[doc(no_inline)]
  pub use crate::state::*;
  #[doc(no_inline)]
  pub use crate::widget;
  #[doc(no_inline)]
  pub use crate::widget::*;
  #[doc(no_inline)]
  pub use crate::widget_children::*;
  #[doc(no_inline)]
  pub use crate::widget_tree::{BoxClamp, LayoutInfo, Layouter, WidgetId};
  #[doc(no_inline)]
  pub use crate::window::Window;
  #[doc(no_inline)]
  pub use ::ribir_painter::*;
  pub use log;
  #[doc(no_inline)]
  pub use ribir_algo::CowArc;
  #[doc(no_inline)]
  pub use ribir_macros::{include_svg, widget, Declare, Lerp, MultiChild, SingleChild, Template};
  #[doc(hidden)]
  pub use rxrust::prelude::*;
}

pub mod test;
