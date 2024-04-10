use crate::prelude::*;

/// A widget that insets its child by the given padding.
#[derive(SingleChild, Query, Clone, Default)]
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
    let mut child_layouter = ctx.assert_single_child_layouter();

    let mut size = child_layouter.perform_widget_layout(child_clamp);
    if child_layouter.has_child() {
      // Expand the size, so the child have padding.
      size = clamp.clamp(size + thickness);
      child_layouter.update_size(child, size);

      // Update child's children position, let they have a correct position after
      // expanded with padding. padding.
      let mut grandson_layouter = child_layouter.into_first_child_layouter();
      while let Some(mut l) = grandson_layouter {
        if let Some(pos) = l.box_pos() {
          let pos = pos + Vector::new(self.padding.left, self.padding.top);
          l.update_position(pos);
        }

        grandson_layouter = l.into_next_sibling()
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

  fn smoke() -> impl WidgetBuilder {
    fn_widget! {
      @MockMulti {
        padding: EdgeInsets::only_left(1.),
        @MockBox {
           size: Size::new(100., 100.),
        }
      }
    }
  }
  widget_layout_test!(
    smoke,
    // padding widget
    { path = [0], width == 101., height == 100.,}
    // MockMulti widget
    { path = [0, 0], width == 101., height == 100., }
    // MockBox
    { path = [0, 0, 0], x == 1., width == 100., height == 100.,}
  );
}
