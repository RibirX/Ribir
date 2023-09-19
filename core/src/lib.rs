#![feature(decl_macro)]
#![cfg_attr(test, feature(mutex_unpoison, test))]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;

pub mod animation;
pub mod builtin_widgets;
mod context;
pub mod data_widget;
mod state;
pub(crate) mod widget_tree;

pub mod clipboard;
pub mod declare;
pub mod events;
pub mod pipe;
pub mod ticker;
pub mod timer;
pub mod widget;
pub mod widget_children;
pub mod window;
pub use rxrust;

pub mod prelude {
  pub use crate::animation::*;
  #[doc(no_inline)]
  pub use crate::builtin_widgets::*;
  #[doc(no_inline)]
  pub use crate::context::*;
  #[doc(no_inline)]
  pub use crate::data_widget::{AnonymousWrapper, DataWidget};
  #[doc(no_inline)]
  pub use crate::declare::*;
  #[doc(no_inline)]
  pub use crate::events::*;
  #[doc(no_inline)]
  pub use crate::pipe::Pipe;
  #[doc(no_inline)]
  pub use crate::state::*;
  #[doc(no_inline)]
  pub use crate::widget;
  #[doc(no_inline)]
  pub use crate::widget::{Any, Compose, FnWidget, HitTest, Query, Render, TypeId, Widget};
  #[doc(no_inline)]
  pub use crate::widget_children::*;
  #[doc(no_inline)]
  pub use crate::widget_tree::{BoxClamp, LayoutInfo, Layouter, WidgetId};
  #[doc(no_inline)]
  pub use crate::window::Window;
  pub use log;
  #[doc(no_inline)]
  pub use ribir_algo::CowArc;
  pub use ribir_geom::*;
  #[doc(no_inline)]
  pub use ribir_macros::{
    ctx, fn_widget, include_svg, map_writer, pipe, rdl, ribir_expanded_ಠ_ಠ, set_build_ctx,
    split_writer, watch, Declare2, Lerp, MultiChild, SingleChild, Template,
  };
  #[doc(no_inline)]
  pub use ribir_painter::*;
  #[doc(no_inline)]
  pub use ribir_text::*;
  #[doc(hidden)]
  pub use rxrust::prelude::*;
}

pub mod test_helper;
