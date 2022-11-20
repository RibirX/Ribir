use ribir_core::{impl_query_self_only, prelude::*};

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
  use ribir_core::test::{expect_layout_result, ExpectRect, LayoutTestItem};

  use super::*;
  use crate::prelude::*;

  #[test]
  fn smoke() {
    let widget = widget! {
      TransformBox {
        matrix: Transform::new(2., 0., 0., 2., 0., 0.),
        SizedBox { size: Size::new(100., 100.) }
      }
    };

    expect_layout_result(
      widget,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(Size::new(200., 200.)),
      }],
    );
  }
}
