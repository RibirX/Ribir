use crate::{prelude::*, wrap_render::*};

/// A wrapper that applies additional constraint clamping to its child.
///
/// This is a built-in `FatObj` field. Setting the `clamp` field attaches a
/// `ConstrainedBox` which constrains the child's min/max sizes.
///
/// # Example
///
/// Constrain a container to a maximum width of 100.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(200., 50.), // This will be constrained to width 100.
///   background: Color::RED,
///   clamp: BoxClamp::max_width(100.),
/// };
/// ```
#[derive(Clone, Default)]
pub struct ConstrainedBox {
  pub clamp: BoxClamp,
}

impl Declare for ConstrainedBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(ConstrainedBox);

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

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
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
