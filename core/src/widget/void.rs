use crate::prelude::*;

/// A virtual widget use to help write code when you need a widget as a virtual
/// node in `widget!` macro, or hold a place in tree. When it have a child
/// itself will be dropped when build tree, otherwise as a render widget but do
/// nothing.
#[derive(Declare)]
pub struct Void;

impl ComposeSingleChild for Void {
  #[inline]
  fn compose_single_child(_: Stateful<Self>, child: Widget, _: &mut BuildCtx) -> Widget { child }
}

impl Render for Void {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn only_sized_by_parent(&self) -> bool { true }
}
