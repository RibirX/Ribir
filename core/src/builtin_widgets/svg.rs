use crate::prelude::*;

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.clamp(self.size) }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    painter.draw_svg(self);
  }
}
