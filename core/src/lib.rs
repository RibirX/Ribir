#![feature(test, decl_macro, marker_trait_attr, min_specialization)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;
extern crate widget_derive;

mod application;
pub mod window;

mod context;
pub mod declare;
pub mod events;
pub mod widget;

pub mod prelude {
  #[doc(no_inline)]
  pub use crate::application::Application;
  #[doc(no_inline)]
  pub use crate::context::*;
  #[doc(no_inline)]
  pub use crate::declare::{Declare, DeclareBuilder};
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
  pub use widget_derive::{declare, Declare, MultiChildWidget, SingleChildWidget};
}

pub mod test;
