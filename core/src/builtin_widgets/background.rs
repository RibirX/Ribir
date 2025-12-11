use wrap_render::WrapRender;

use super::*;

/// A widget that utilizes the background brush to paint a background box based
/// on the layout size. If a `Radius` is provided, the corners of the box will
/// be rounded.
#[derive(Default, Clone)]
pub struct Background {
  /// The background of the box.
  pub background: Brush,
}

impl Declare for Background {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for Background {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();

    if !size.is_empty() {
      let rect = Rect::from_size(size);
      let (provider_ctx, mut painter) = ctx.provider_ctx_and_box_painter();
      let old_brush = painter.fill_brush().clone();

      painter.set_fill_brush(self.background.clone());
      if let Some(radius) = Provider::of::<Radius>(provider_ctx) {
        painter.rect_round(&rect, &radius, true);
      } else {
        painter.rect(&rect, true);
      }
      painter.fill();

      painter.set_fill_brush(old_brush);
    }
    host.paint(ctx);
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    let visual_box = host.visual_box(ctx);
    let size = ctx.box_size()?;
    if visual_box.is_none() {
      Some(Rect::from_size(size))
    } else {
      visual_box.map(|rect| rect.union(&Rect::from_size(size)))
    }
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}

impl_compose_child_for_wrap_render!(Background);
