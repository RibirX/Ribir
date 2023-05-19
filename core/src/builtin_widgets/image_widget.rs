use crate::{impl_query_self_only, prelude::*};
use ribir_geom::{Rect, Size};

impl Render for ShareResource<PixelImage> {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    Size::new(self.width() as f32, self.height() as f32)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let rect = Rect::from_size(size);
    let painter = ctx.painter();
    if self.width() > size.width as u32 || self.height() > size.height as u32 {
      painter.clip(Path::rect(&rect));
    }
    painter.set_brush(self.clone()).rect(&rect).fill();
  }
}

impl Query for ShareResource<PixelImage> {
  impl_query_self_only!();
}
