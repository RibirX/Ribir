use crate::prelude::*;
use std::ptr::NonNull;

pub struct BuildCtx {
  pub(crate) tree: NonNull<widget_tree::WidgetTree>,
}

impl BuildCtx {
  #[inline]
  pub(crate) fn new(tree: NonNull<widget_tree::WidgetTree>, current: WidgetId) -> Self {
    Self { tree }
  }
}
