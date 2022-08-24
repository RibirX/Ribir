use super::WidgetCtxImpl;
use crate::prelude::{widget_tree::WidgetTree, WidgetId};

pub struct LifeCycleCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a mut WidgetTree,
}

impl<'a> WidgetCtxImpl for LifeCycleCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn widget_tree(&self) -> &WidgetTree { &self.tree }
}
