use crate::{impl_query_self_only, prelude::*};

/// A virtual widget use to help write code when you need a widget as a virtual
/// node in `widget!` macro, or hold a place in tree. When it have a child
/// itself will be dropped when build tree, otherwise as a render widget but do
/// nothing.
#[derive(SingleChild, Declare2)]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child_layouter()
      .map_or_else(Size::zero, |mut l| l.perform_widget_layout(clamp))
  }

  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}

impl_query_self_only!(Void);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use ribir_dev_helper::*;
  extern crate test;
  use test::Bencher;

  /// Measures the time required to build and layout an `Void` widget, and other
  /// widget can subtract this time to obtain their own real time
  fn base() -> impl WidgetBuilder { fn_widget!(Void) }
  widget_bench!(base);
}
