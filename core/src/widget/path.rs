use crate::{impl_query_self_only, prelude::*};

impl Render for Path {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    self.box_rect().max().to_vector().to_size()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().paint_path(self.clone()); }
}

impl Query for Path {
  impl_query_self_only!();
}
