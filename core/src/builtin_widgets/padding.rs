use crate::prelude::*;

/// A widget that insets its child by the given padding.
#[derive(SingleChild, Clone, Default)]
pub struct Padding {
  pub padding: EdgeInsets,
}

impl Declare for Padding {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Render for Padding {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = match ctx.single_child() {
      Some(c) => c,
      None => return Size::zero(),
    };

    let thickness = self.padding.thickness();
    let zero = Size::zero();
    let min = (clamp.min - thickness).max(zero);
    let max = (clamp.max - thickness).max(zero);
    // Shrink the clamp of child.
    let child_clamp = BoxClamp { min, max };
    ctx.force_child_relayout(child);
    let child = ctx.assert_single_child();

    let mut size = ctx.perform_child_layout(child, child_clamp);
    if child.first_child(ctx.tree).is_some() {
      // Expand the size, so the child have padding.
      size = clamp.clamp(size + thickness);
      ctx.update_size(child, size);

      // Update child's children position, let they have a correct position after
      // expanded with padding. padding.
      let mut ctx = LayoutCtx { id: child, tree: ctx.tree };
      let (ctx, grandson) = ctx.split_children();
      for g in grandson {
        if let Some(pos) = ctx.widget_box_pos(g) {
          let pos = pos + Vector::new(self.padding.left, self.padding.top);
          ctx.update_position(g, pos);
        }
      }
    }

    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
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
    // MockMulti widget
    LayoutCase::new(&[0, 0]).with_size(Size::new(101., 100.)),
    // MockBox
    LayoutCase::new(&[0, 0, 0])
      .with_size(Size::new(100., 100.))
      .with_x(1.)
  );
}
