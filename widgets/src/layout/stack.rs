use ribir_core::prelude::*;

/// A widget that overlap children align with left top.
#[derive(MultiChild, Declare)]
pub struct Stack {
  #[declare(default)]
  fit: StackFit,
}

/// How to size the non-positioned children of a [Stack].
#[derive(Default)]
pub enum StackFit {
  /// The constraints passed to the stack from its parent are loosened.
  ///
  /// For example, if the stack has constraints that force it to 350x600, then
  /// this would allow the non-positioned children of the stack to have any
  /// width from zero to 350 and any height from zero to 600.
  #[default]
  Loose,

  /// The constraints passed to the stack from its parent are tightened to the
  /// biggest size allowed.
  ///
  /// For example, if the stack has loose constraints with a width in the range
  /// 10 to 100 and a height in the range 0 to 600, then the non-positioned
  /// children of the stack would all be sized as 100 pixels wide and 600 high.
  Expand,

  /// The constraints passed to the stack from its parent are passed unmodified
  /// to the non-positioned children.
  ///
  /// For example, if a [Stack] is an [Expanded] child of a [Row], the
  /// horizontal constraints will be tight and the vertical constraints will be
  /// loose.
  Passthrough,
}

impl Render for Stack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let stack_clamp = match self.fit {
      StackFit::Loose => clamp.loose(),
      StackFit::Expand if clamp.max.is_finite() => BoxClamp { min: clamp.max, max: clamp.max },
      _ => clamp,
    };

    let mut size = ZERO_SIZE;
    let (ctx, children) = ctx.split_children();
    for c in children {
      let child_size = ctx.perform_child_layout(c, stack_clamp);
      size = size.max(child_size);
    }
    clamp.clamp(size)
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;
  const FIVE: Size = Size::new(5., 5.);

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @Stack {
        @SizedBox { size: Size::new(1., 1.) }
        @SizedBox { size: FIVE }
      }
    }),
    LayoutCase::default().with_size(FIVE)
  );
}
