use crate::{prelude::*, impl_query_self_only};

#[derive(SingleChild, Declare, Clone)]
pub struct TransformBox {
  pub matrix: Transform,
}

impl Render for TransformBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.single_child().map_or_else(Size::zero, |c| {
      self.matrix.inverse().map_or_else(Size::zero, |t| {
        let min_box = t.outer_transformed_box(&Box2D::from_size(clamp.min));
        let min = min_box.size();

        let max_box = t.outer_transformed_box(&Box2D::from_size(clamp.max));
        let max = max_box.size();

        let child_clamp = BoxClamp { min, max };
        let size = ctx.perform_child_layout(c, child_clamp);
        let rect = self.matrix.outer_transformed_rect(&Rect::from_size(size));
        rect.size
      })
    })
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for TransformBox {
  impl_query_self_only!();
}

impl TransformBox {
  #[inline]
  pub fn new(matrix: Transform) -> Self { Self { matrix } }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn smoke() {
    let widget = widget! {
      TransformBox {
        matrix: Transform::new(2., 0., 0., 2., 0., 0.),

        SizedBox {
          size: Size::new(100., 100.)
        }
      }
    };

    let (rect, _) =
      widget_and_its_children_box_rect(widget.into_widget(), Size::new(800., 800.));

    assert_eq!(rect, Rect::from_size(Size::new(200., 200.)));
  }
}
