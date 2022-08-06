use crate::prelude::{widget_tree::WidgetTree, Painter, WidgetId};

use super::WidgetCtxImpl;

pub struct PaintingCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
  pub(crate) painter: &'a mut Painter,
}

impl<'a> PaintingCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree, painter: &'a mut Painter) -> Self {
    Self { id, tree, painter }
  }
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &crate::prelude::widget_tree::WidgetTree { &self.tree }
}
