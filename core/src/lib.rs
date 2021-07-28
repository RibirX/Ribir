#![allow(incomplete_features)]
#![feature(
  specialization,
  test,
  box_into_inner,
  decl_macro,
  negative_impls,
  linked_list_cursors,
  trivial_bounds
)]
#[macro_use]
extern crate bitflags;
extern crate lazy_static;
extern crate widget_derive;

mod application;
mod render;
pub mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render::*;
  pub use crate::widget;
  pub use crate::widget::{build_ctx::BuildCtx, widget_tree::WidgetId, *};
  pub use canvas::*;
  pub use rxrust::prelude::*;
  pub use widget::layout::{MultiChild, SingleChild};
  pub use widget_derive::{
    stateful, CombinationWidget, MultiChildWidget, RenderWidget, SingleChildWidget, StatePartialEq,
    Widget,
  };
}

#[cfg(test)]
pub mod test;
