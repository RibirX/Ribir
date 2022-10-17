use crate::impl_query_self_only;
use crate::prelude::*;

#[derive(Declare, Default, Clone, SingleChild)]
pub struct Opacity {
  #[declare(builtin)]
  pub opacity: f32,
}

impl Query for Opacity {
  impl_query_self_only!();
}

impl Render for Opacity {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = ctx.single_child().expect("Alpha must have one child");
    ctx.perform_child_layout(child, clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_alpha(self.opacity); }

  fn only_sized_by_parent(&self) -> bool { false }
}
