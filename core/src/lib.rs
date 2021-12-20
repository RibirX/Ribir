#![feature(
  test,
  decl_macro,
  negative_impls,
  cell_filter_map,
  linked_list_cursors,
  trivial_bounds,
  auto_traits
)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;
extern crate widget_derive;

mod application;
mod declare;
mod render;
pub mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::declare::{Declare, DeclareBuilder};
  pub use crate::render::*;
  pub use crate::widget;
  pub use crate::widget::{build_ctx::BuildCtx, widget_tree::WidgetId, *};
  pub use canvas::prelude::*;
  pub use rxrust::prelude::*;
  pub use widget::layout::{MultiChild, SingleChild};
  pub use widget_derive::{
    declare, stateful, CombinationWidget, Declare, MultiChildWidget, RenderWidget,
    SingleChildWidget, StatePartialEq,
  };
}

pub mod test;
