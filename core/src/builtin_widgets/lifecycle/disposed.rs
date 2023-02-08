use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

type DisposedCallback = dyn for<'r> FnMut(LifeCycleCtx<'r>);
#[derive(Declare)]
pub struct DisposedListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(LifeCycleCtx<'r>), wrap_fn = RefCell::new))]
  pub on_disposed: RefCell<Box<DisposedCallback>>,
}

impl_lifecycle!(DisposedListener, on_disposed);
