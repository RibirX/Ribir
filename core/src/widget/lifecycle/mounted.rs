use crate::{
  impl_lifecycle, impl_query_self_only,
  prelude::{data_widget::compose_child_as_data_widget, *},
};

/// Listener perform when its child widget add to the widget tree.
#[derive(Declare)]
pub struct MountedListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(LifeCycleCtx<'r>)))]
  pub on_mounted: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>,
}
impl_lifecycle!(MountedListener, on_mounted);
