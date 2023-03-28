use crate::{impl_query_self_only, prelude::*};
use ribir_painter::{Brush, Point, Rect, ShallowImage, Size, TileMode};

impl Render for ShallowImage {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
    let (w, h) = self.size();
    Size::new(w as f32, h as f32)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let painter = ctx.painter();
    let img_brush = Brush::Image {
      img: self.clone(),
      tile_mode: TileMode::COVER_BOTH,
    };
    painter.rect(&Rect::new(Point::zero(), size));
    painter.set_brush(img_brush).fill();
  }
}

impl Query for ShallowImage {
  impl_query_self_only!();
}
