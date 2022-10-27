use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

#[derive(Copy, Clone, PartialEq)]
pub enum DisposedType {
  /// The event fires with Drop when the widget is drop
  Drop,
  /// The event fires with Refresh when the widget is rebuild(with the
  /// same Key),usually happen in the ExprWidget. when the data change will
  /// trigger the ExprWidget rebuild。 the new widget with the key appear
  /// before will trigger the mounted event with Refresh.
  Replaced(WidgetId),
}

#[derive(Declare)]
pub struct DisposedListener {
  #[declare(builtin, convert=listener_callback(for<'r> FnMut(LifeCycleCtx<'r>, DisposedType)))]
  pub disposed: RefCell<Box<dyn for<'r> FnMut(LifeCycleCtx<'r>, DisposedType)>>,
}

impl_lifecycle!(DisposedListener, disposed);
