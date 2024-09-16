use crate::prelude::*;

/// A virtual widget use to help write code when you need a widget as a virtual
/// node in `widget!` macro, or hold a place in tree. When it have a child
/// itself will be dropped when build tree, otherwise as a render widget but do
/// nothing.
#[derive(SingleChild, Declare)]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .perform_single_child_layout(clamp)
      .unwrap_or(clamp.min)
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}
