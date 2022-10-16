use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Clone, Declare)]
pub struct Clip
{
  path: Path,
}

impl Render for Clip  {
  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = ctx.single_child().expect("Clip must have one child.");
    ctx.perform_child_layout(child, clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    ctx.painter().clip(self.path.clone());
  }
}

impl Query for Clip {
  impl_query_self_only!();
}
