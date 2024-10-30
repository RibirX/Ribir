use crate::{prelude::*, wrap_render::*};

/// A widget that insets its child by the given padding.
#[derive(Default)]
pub struct Padding {
  pub padding: EdgeInsets,
}

impl Declare for Padding {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(Padding);

impl WrapRender for Padding {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let thickness = self.padding.thickness();
    let zero = Size::zero();
    // Reset children position before layout
    let (ctx, children) = ctx.split_children();
    for c in children {
      ctx.update_position(c, Point::zero());
    }

    let min = (clamp.min - thickness).max(zero);
    let max = (clamp.max - thickness).max(zero);
    // Shrink the clamp of child.
    let child_clamp = BoxClamp { min, max };
    let mut size = host.perform_layout(child_clamp, ctx);

    size = clamp.clamp(size + thickness);

    let (ctx, children) = ctx.split_children();
    // Update the children's positions to ensure they are correctly positioned after
    // expansion with padding.
    for c in children {
      if let Some(pos) = ctx.widget_box_pos(c) {
        let pos = pos + Vector::new(self.padding.left, self.padding.top);
        ctx.update_position(c, pos);
      }
    }

    size
  }
}

impl Padding {
  #[inline]
  pub fn new(padding: EdgeInsets) -> Self { Self { padding } }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @MockMulti {
        padding: EdgeInsets::only_left(1.),
        @MockBox {
           size: Size::new(100., 100.),
        }
      }
    }),
    // padding widget
    LayoutCase::default().with_size(Size::new(101., 100.)),
    // MockBox
    LayoutCase::new(&[0, 0])
      .with_size(Size::new(100., 100.))
      .with_x(1.)
  );
}
