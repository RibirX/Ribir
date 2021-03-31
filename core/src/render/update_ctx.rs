use crate::prelude::*;

pub struct UpdateCtx<'a> {
  render_tree: &'a mut render_tree::RenderTree,
  rid: RenderId,
}

impl<'a> UpdateCtx<'a> {
  pub(crate) fn new(rid: RenderId, render_tree: &'a mut render_tree::RenderTree) -> Self {
    Self { render_tree, rid }
  }
  /// Mark this render object needs relayout, and spread up to an ancestor which
  /// its size only effected by parent.
  #[inline]
  pub fn mark_needs_layout(&mut self) { self.rid.mark_needs_layout(self.render_tree); }
}
