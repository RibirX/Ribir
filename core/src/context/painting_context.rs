use crate::prelude::{widget_tree::WidgetTree, LayoutStore, Painter, WidgetId};

use super::WidgetCtxImpl;

pub struct PaintingCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
  pub(crate) layout_store: &'a LayoutStore,
  pub(crate) painter: &'a mut Painter,
}

impl<'a> PaintingCtx<'a> {
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &crate::prelude::widget_tree::WidgetTree { &self.tree }

  fn layout_store(&self) -> &crate::prelude::LayoutStore { self.layout_store }
}
