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
      .single_child()
      .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn only_sized_by_parent(&self) -> bool { true }
}

impl crate::prelude::ComposeSingleChild for Void {
  #[inline]
  fn compose_single_child(_: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget { child }
}

impl Query for Void {
  impl_query_self_only!();
}
