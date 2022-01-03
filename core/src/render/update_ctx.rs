use crate::prelude::*;

pub struct UpdateCtx<'a> {
  r_tree: &'a render_tree::RenderTree,
  rid: RenderId,
  layout_store: &'a mut layout_store::LayoutStore,
}

impl<'a> UpdateCtx<'a> {
  pub(crate) fn new(
    rid: RenderId,
    render_tree: &'a render_tree::RenderTree,
    layout_store: &'a mut layout_store::LayoutStore,
  ) -> Self {
    Self {
      r_tree: render_tree,
      rid,
      layout_store,
    }
  }
  /// Mark this render object needs relayout, and spread up to an ancestor which
  /// its size only effected by parent.
  #[inline]
  pub fn mark_needs_layout(&mut self) { self.layout_store.mark_needs_layout(self.rid, self.r_tree) }
}
