use ribir_core::prelude::*;

/// a widget that imposes additional constraints clamp on its child.
#[derive(SingleChild, Declare, Clone)]
pub struct ConstrainedBox {
  pub clamp: BoxClamp,
}

impl Render for ConstrainedBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.clamp(self.clamp.max);
    let min = clamp.clamp(self.clamp.min);
    ctx.assert_perform_single_child_layout(BoxClamp { min, max })
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_layout_test! (
    outside_fixed_clamp,
    fn_widget! {
      @SizedBox {
        size: Size::new(50., 50.),
        @ConstrainedBox {
          clamp: BoxClamp::fixed_size(Size::new(40., 40.)),
          @Void {}
        }
      }
    },
    {path =[0,0,0], width == 50., height == 50.,}
  );

  widget_layout_test!(
    expand_one_axis,
    fn_widget! {
      @Container {
        size: Size::new(256., 50.),
        @ConstrainedBox {
          clamp: BoxClamp::EXPAND_X,
          @Container {
            size: Size::new(128., 20.),
          }
        }
      }
    },
    { path = [0, 0], width==256., height == 20. ,}
  );

  widget_layout_test!(
    expand_both,
    fn_widget! {
      @Container {
        size: Size::new(256., 50.),
        @ConstrainedBox {
          clamp: BoxClamp::EXPAND_BOTH,
          @Container {
            size: Size::new(128., 20.),
          }
        }
      }
    },
    { path = [0, 0], width == 256., height == 50.,}
  );
}
