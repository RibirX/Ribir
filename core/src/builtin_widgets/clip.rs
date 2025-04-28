use crate::prelude::*;

#[derive(SingleChild, Declare)]
pub struct Clip {
  pub clip_path: Path,
}

impl Render for Clip {
  fn size_affected_by_child(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp);
    self
      .clip_path
      .bounds(None)
      .max()
      .to_tuple()
      .into()
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().clip(self.clip_path.clone().into()); }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let clip_rect = self.clip_path.bounds(None);
    ctx.clip(clip_rect);
    Some(clip_rect)
  }
}
