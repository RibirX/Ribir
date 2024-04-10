#![cfg_attr(feature = "nightly", feature(closure_track_caller))]
#[macro_use]
extern crate bitflags;
extern crate lazy_static;

pub mod animation;
pub mod builtin_widgets;
mod context;
pub mod data_widget;
pub(crate) mod render_helper;
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
pub mod overlay;

pub mod prelude {
  pub use log;
  #[doc(no_inline)]
  pub use ribir_algo::CowArc;
  pub use ribir_geom::*;
  #[doc(no_inline)]
  pub use ribir_macros::*;
  #[doc(no_inline)]
  pub use ribir_text::*;
  #[doc(hidden)]
  pub use rxrust::prelude::*;

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
  pub use crate::overlay::{Overlay, OverlayCloseHandle};
  #[doc(no_inline)]
  pub use crate::pipe::{BoxPipe, FinalChain, MapPipe, ModifiesPipe, Pipe};
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
  pub use crate::{
    animation::*,
    ticker::{Duration, Instant},
  };
}

pub mod test_helper;
