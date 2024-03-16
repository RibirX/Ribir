use crate::prelude::*;

#[derive(Query, Clone, SingleChild)]
pub struct Opacity {
  pub opacity: f32,
}

impl Declare for Opacity {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Default for Opacity {
  #[inline]
  fn default() -> Self { Self { opacity: 1.0 } }
}

impl Render for Opacity {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_alpha(self.opacity); }

  fn only_sized_by_parent(&self) -> bool { false }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}
