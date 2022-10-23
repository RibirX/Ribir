use crate::widget_tree::{WidgetId, WidgetTree};

use super::WidgetCtxImpl;

pub struct TreeCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
}

impl<'a> TreeCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree) -> Self { Self { id, tree } }
}

impl<'a> WidgetCtxImpl for TreeCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &WidgetTree { &self.tree }
}
