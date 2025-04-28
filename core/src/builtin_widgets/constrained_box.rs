use crate::{prelude::*, wrap_render::*};

/// a widget that imposes additional constraints clamp on its child.
#[derive(Clone, Default)]
pub struct ConstrainedBox {
  pub clamp: BoxClamp,
}

impl Declare for ConstrainedBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(ConstrainedBox, DirtyPhase::Layout);

impl WrapRender for ConstrainedBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.clamp(self.clamp.max);
    let min = clamp.clamp(self.clamp.min);
    host.perform_layout(BoxClamp { min, max }, ctx)
  }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    let is_fixed = self.clamp.min == self.clamp.max;
    if is_fixed { false } else { host.size_affected_by_child() }
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    outside_fixed_clamp,
    WidgetTester::new(fn_widget! {
      @ConstrainedBox {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.)),
        @Void {
          clamp: BoxClamp::fixed_size(Size::new(40., 40.))
        }
      }
    }),
    LayoutCase::new(&[0]).with_size(Size::new(50., 50.))
  );
}
