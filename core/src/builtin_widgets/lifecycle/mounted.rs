use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

#[derive(Copy, Clone, PartialEq)]
pub enum MountedType {
  /// The mounted event fires with New when an widget is first build
  New,
  /// The mounted event fires with Refresh when the widget is rebuild(with the
  /// same Key),usually happen in the ExprWidget. when the data change will
  /// trigger the ExprWidget rebuild。 the new widget with the key appear
  /// before will trigger the mounted event with Refresh.
  Replace(WidgetId),
}

/// Listener perform when its child widget add to the widget tree.
#[derive(Declare)]
pub struct MountedListener {
  #[declare(builtin, convert=listener_callback(for<'r> FnMut(LifeCycleCtx<'r>, MountedType)))]
  pub mounted: RefCell<Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, MountedType)>>,
}

impl_lifecycle!(MountedListener, mounted);
