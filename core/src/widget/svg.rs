use crate::{impl_query_self_only, prelude::*};

#[derive(Declare)]
pub struct Svg {
  bytes: Vec<u8>,
}

impl Compose for Svg {
  fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this }
      ExprWidget { expr: SvgRender::new(&this.bytes) }
    }
  }
}

impl Render for SvgRender {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    self.paths.iter().for_each(|c| {
      if let Some(b) = c.brush.as_ref() {
        painter.set_brush(b.clone());
      }
      painter
        .apply_transform(&c.transform)
        .paint_path(c.path.clone());
    });
  }
}

impl Query for SvgRender {
  impl_query_self_only!();
}
