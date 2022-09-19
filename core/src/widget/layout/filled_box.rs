use crate::{impl_query_self_only, prelude::*};
#[derive(Declare, SingleChild)]
pub struct FilledBox {}
impl Query for FilledBox {
  impl_query_self_only!();
}

impl Render for FilledBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if let Some(child) = ctx.single_child() {
      ctx.perform_child_layout(child, clamp);
    }
    clamp.max
  }

  fn paint(&self, _ctx: &mut PaintingCtx) {}

  fn only_sized_by_parent(&self) -> bool { true }
}
