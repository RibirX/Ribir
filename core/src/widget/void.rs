use crate::{impl_query_self_only, prelude::*};

/// A virtual widget use to help write code when you need a widget as a virtual
/// node in `widget!` macro, or hold a place in tree. When it have a child
/// itself will be dropped when build tree, otherwise as a render widget but do
/// nothing.
#[derive(Declare)]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { unreachable!() }

  fn paint(&self, _: &mut PaintingCtx) { unreachable!() }

  fn only_sized_by_parent(&self) -> bool { unreachable!() }
}

impl Query for Void {
  impl_query_self_only!();
}
