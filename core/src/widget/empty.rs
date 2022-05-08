use crate::prelude::*;

/// Widget only use to help write code as a empty root, and will be drooped
/// after widget tree build.
#[derive(Declare, SingleChildWidget)]
pub struct Empty;

impl Render for Empty {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    // todo: should drop this widget after tree build.
    // unreachable!()
    ctx.perform_child_layout(ctx.single_child().unwrap(), clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    // todo: should drop this widget after tree build.
    // unreachable!()
  }
}
