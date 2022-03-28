#![feature(test, decl_macro, marker_trait_attr, min_specialization)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;
extern crate widget_derive;

pub mod animation;
mod application;
mod context;
pub mod declare;
pub mod events;
pub mod widget;
pub mod window;

pub mod prelude {
  pub use crate::animation::*;
  #[doc(no_inline)]
  pub use crate::application::Application;
  #[doc(no_inline)]
  pub use crate::context::*;
  #[doc(no_inline)]
  pub use crate::declare::*;
  #[doc(no_inline)]
  pub use crate::events::*;
  #[doc(no_inline)]
  pub use crate::widget;
  #[doc(no_inline)]
  pub use crate::widget::{widget_tree::WidgetId, *};
  #[doc(no_inline)]
  pub use crate::window::Window;
  #[doc(no_inline)]
  pub use ::painter::*;
  #[doc(no_inline)]
  pub use algo::CowRc;
  #[doc(hidden)]
  pub use rxrust::prelude::*;
  #[doc(no_inline)]
  pub use widget::layout::{MultiChild, SingleChild};
  #[doc(no_inline)]
  pub use widget_derive::{widget, Declare, MultiChildWidget, SingleChildWidget};
}

pub mod test;
