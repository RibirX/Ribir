use super::WidgetCtxImpl;
use crate::{
  prelude::{Painter, WidgetId},
  widget::WidgetTree,
};

pub struct PaintingCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
  pub(crate) painter: &'a mut Painter,
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> PaintingCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree, painter: &'a mut Painter) -> Self {
    Self { id, tree, painter }
  }
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }
}
