use crate::prelude::*;

impl RenderWidget for Path {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    self.box_rect().max().to_vector().to_size()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().paint_path(self.clone()); }
}
