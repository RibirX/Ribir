use crate::prelude::*;

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    painter.draw_svg(self);
  }
}

impl Query for Svg {
  crate::widget::impl_query_self_only!();
}
