use crate::prelude::*;

#[derive(SingleChild, Declare)]
pub struct Clip {
  pub clip_path: Path,
}

impl Render for Clip {
  fn only_sized_by_parent(&self) -> bool { true }

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
}
