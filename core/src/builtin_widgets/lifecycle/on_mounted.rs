use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

type MountedCallback = dyn for<'r> FnMut(LifeCycleCtx<'r>);
/// Listener perform when its child widget add to the widget tree.
#[derive(Declare)]
pub struct MountedListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(LifeCycleCtx<'r>), wrap_fn=RefCell::new))]
  pub on_mounted: RefCell<Box<MountedCallback>>,
}

impl_lifecycle!(MountedListener, on_mounted);
