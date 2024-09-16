use crate::{prelude::*, wrap_render::WrapRender};

#[derive(Declare, Clone)]
pub struct IgnorePointer {
  #[declare(default = true)]
  pub ignore: bool,
}

impl<'c> ComposeChild<'c> for IgnorePointer {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for IgnorePointer {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &HitTestCtx, pos: Point) -> HitTest {
    if self.ignore { HitTest { hit: false, can_hit_child: false } } else { host.hit_test(ctx, pos) }
  }
}

impl IgnorePointer {
  #[inline]
  pub fn new(ignore: bool) -> Self { Self { ignore } }
}
