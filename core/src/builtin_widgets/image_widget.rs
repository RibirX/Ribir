use crate::{impl_query_self_only, prelude::*};
use ribir_painter::{Rect, Size};

impl Render for ShareResource<PixelImage> {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    Size::new(self.width() as f32, self.height() as f32)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    ctx
      .painter()
      .set_brush(self.clone())
      .rect(&Rect::from_size(size))
      .fill();
  }
}

impl Query for ShareResource<PixelImage> {
  impl_query_self_only!();
}
