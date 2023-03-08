use std::error::Error;

use crate::{impl_query_self_only, prelude::*};

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    self.0.paths.iter().for_each(|c| {
      if let Some(b) = c.brush.as_ref() {
        painter.set_brush(b.clone());
      }
      painter.fill_path(c.path.clone());
    });
  }
}

impl Query for Svg {
  impl_query_self_only!();
}
