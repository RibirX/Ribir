use crate::prelude::*;

/// A widget use to help write code when you need a widget as a empty node in
/// `widget!` macro, or hold a place in tree.
///
/// When it have a child itself will be dropped when build tree, otherwise as a
/// render widget but do nothing.
#[derive(Declare)]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .perform_single_child_layout(clamp)
      .unwrap_or(clamp.min)
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

impl<'c> ComposeChild<'c> for Void {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> { child }
}
