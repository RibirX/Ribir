use crate::{impl_query_self_only, prelude::*};

/// A widget that insets its child by the given padding.
#[derive(SingleChild, Clone, Declare)]
pub struct Padding {
  #[declare(builtin)]
  pub padding: EdgeInsets,
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

    let size = child_layouter.perform_widget_layout(child_clamp);

    // Expand the size, so the child have padding.
    let size = clamp.clamp(size + thickness);
    child_layouter.update_size(child, size);

    // Update child's children position, let they have a correct position after
    // expanded with padding. padding.
    let mut layouter = child_layouter.into_first_child_layouter();
    while let Some(mut l) = layouter {
      if let Some(pos) = l.layout_pos() {
        let pos = pos + Vector::new(self.padding.left, self.padding.top);
        l.update_position(pos);
      }

      layouter = l.into_next_sibling()
    }

    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Padding {
  impl_query_self_only!();
}

impl Padding {
  #[inline]
  pub fn new(padding: EdgeInsets) -> Self { Self { padding } }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;
  #[test]
  fn smoke() {
    let widget = widget! {
      MockMulti {
        padding: EdgeInsets::only_left(1.),
        Container {
           size: Size::new(100., 100.),
        }
      }
    };
    expect_layout_result(
      widget,
      None,
      &[
        // padding widget
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect {
            width: Some(101.),
            height: Some(100.),
            ..Default::default()
          },
        },
        // MockMulti widget
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect {
            width: Some(101.),
            height: Some(100.),
            ..Default::default()
          },
        },
        // Container
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect {
            x: Some(1.),
            y: Some(0.),
            width: Some(100.),
            height: Some(100.),
          },
        },
      ],
    );
  }
}
