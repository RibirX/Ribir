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

impl_compose_child_for_wrap_render!(ConstrainedBox);

impl WrapRender for ConstrainedBox {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.clamp(self.clamp.max);
    let min = clamp.clamp(self.clamp.min);
    host.perform_layout(BoxClamp { min, max }, ctx)
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

  widget_layout_test!(
    expand_one_axis,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(256., 50.),
        @ConstrainedBox {
          clamp: BoxClamp::EXPAND_X,
          @Container {
            size: Size::new(128., 20.),
          }
        }
      }
    },),
    LayoutCase::new(&[0, 0]).with_size(Size::new(256., 20.))
  );

  widget_layout_test!(
    expand_both,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(256., 50.),
        @ConstrainedBox {
          clamp: BoxClamp::EXPAND_BOTH,
          @Container {
            size: Size::new(128., 20.),
          }
        }
      }
    }),
    LayoutCase::new(&[0, 0]).with_size(Size::new(256., 50.))
  );
}
