use crate::{impl_query_self_only, prelude::*};

/// A virtual widget use to help write code when you need a widget as a virtual
/// node in `widget!` macro, or hold a place in tree. When it have a child
/// itself will be dropped when build tree, otherwise as a render widget but do
/// nothing.
#[derive(Declare)]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child_layouter()
      .map_or_else(Size::zero, |mut l| l.perform_widget_layout(clamp))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl Query for Void {
  impl_query_self_only!();
}
