use crate::{impl_query_self_only, prelude::*};

#[derive(Declare)]
pub struct Svg {
  bytes: Vec<u8>,
}

impl Compose for Svg {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget_try_track! {
      try_track { this }
      ExprWidget {
        expr: match SvgRender::from_svg_bytes(&this.bytes) {
          Ok(reader) => reader.into_widget(),
          Err(err) =>  {
            log::warn!("Parse svg failed: {err}");
            Void.into_widget()
          }
        }
      }
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
