use crate::prelude::*;
use ribir_core::{impl_query_self_only, prelude::*};

#[derive(Default, Declare)]
pub struct Divider {
  #[declare(default=Palette::of(ctx).outline_variant(), convert=into)]
  color: Brush,
  #[declare(default=Direction::Horizontal)]
  direction: Direction,
}

impl Divider {
  pub fn new(color: Brush, direction: Direction) -> Self { Divider { color, direction } }
}

impl Render for Divider {
  fn perform_layout(&self, clamp: BoxClamp, _ctx: &mut LayoutCtx) -> Size {
    if self.direction.is_horizontal() {
      Size::new(clamp.max.width, 1.)
    } else {
      Size::new(1., clamp.max.height)
    }
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let Rect { size, .. } = ctx.box_rect().unwrap();
    let painter = ctx.painter();
    painter.set_brush(self.color.clone());
    painter.rect(&Rect::new(Point::zero(), size));
    painter.fill();
  }
}

impl Query for Divider {
  impl_query_self_only!();
}
