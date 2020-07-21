use crate::prelude::*;
use std::pin::Pin;

pub struct BuildCtx<'a> {
  pub(crate) tree: Pin<&'a mut widget_tree::WidgetTree>,
  #[allow(dead_code)]
  widget: WidgetId,
}

impl<'a> BuildCtx<'a> {
  #[inline]
  pub(crate) fn new(tree: Pin<&'a mut widget_tree::WidgetTree>, widget: WidgetId) -> Self {
    Self { tree, widget }
  }
}
