use crate::{impl_query_self_only, prelude::*};

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    painter.paint_svg(self);
  }
}

impl Query for Svg {
  impl_query_self_only!();
}
