use crate::prelude::{widget_tree::WidgetTree, WidgetId};

use super::WidgetCtxImpl;

pub struct TreeCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
}

impl<'a> TreeCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree) -> Self {
    Self { id, tree }
  }
}

impl<'a> WidgetCtxImpl for TreeCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &crate::prelude::widget_tree::WidgetTree { &self.tree }
}
