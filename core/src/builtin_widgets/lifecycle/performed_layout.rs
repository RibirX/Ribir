use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

type LayouCallback = dyn for<'r> FnMut(LifeCycleCtx<'r>);
#[derive(Declare)]
pub struct PerformedLayoutListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(LifeCycleCtx<'r>), wrap_fn = RefCell::new))]
  pub performed_layout: RefCell<Box<LayouCallback>>,
}
impl_lifecycle!(PerformedLayoutListener, performed_layout);
