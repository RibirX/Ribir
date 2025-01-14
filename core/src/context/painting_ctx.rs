use super::WidgetCtxImpl;
use crate::{
  prelude::{Painter, ProviderCtx, WidgetId},
  widget::WidgetTree,
};

pub struct PaintingCtx<'a> {
  id: WidgetId,
  tree: &'a WidgetTree,
  painter: &'a mut Painter,
  provider_ctx: ProviderCtx,
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> PaintingCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree, painter: &'a mut Painter) -> Self {
    let provider_ctx = if let Some(p) = id.parent(tree) {
      ProviderCtx::collect_from(p, tree)
    } else {
      ProviderCtx::default()
    };

    Self { id, tree, painter, provider_ctx }
  }

  /// Called by the framework when the painting widget is finished.
  #[inline]
  pub(crate) fn finish(&mut self) { self.provider_ctx.pop_providers_for(self.id); }

  #[inline]
  pub(crate) fn switch_to(&mut self, id: WidgetId) { self.id = id; }

  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }

  #[inline]
  pub fn provider_ctx_and_painter(&mut self) -> (&mut ProviderCtx, &mut Painter) {
    (&mut self.provider_ctx, self.painter)
  }
}

impl<'w> AsRef<ProviderCtx> for PaintingCtx<'w> {
  #[inline]
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl<'w> AsMut<ProviderCtx> for PaintingCtx<'w> {
  #[inline]
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}
