use ribir_core::prelude::*;

#[derive(SingleChild, Query, Declare, Clone)]
pub struct TransformBox {
  pub matrix: Transform,
}

impl Render for TransformBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self
      .matrix
      .inverse()
      .map_or_else(Size::zero, |t| {
        let min_box = t.outer_transformed_box(&Box2D::from_size(clamp.min));
        let min = min_box.size();

        let max_box = t.outer_transformed_box(&Box2D::from_size(clamp.max));
        let max = max_box.size();

        let child_clamp = BoxClamp { min, max };

        let mut layouter = ctx.assert_single_child_layouter();
        let size = layouter.perform_widget_layout(child_clamp);
        let rect = self
          .matrix
          .outer_transformed_rect(&Rect::from_size(size));
        rect.size
      })
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl TransformBox {
  #[inline]
  pub fn new(matrix: Transform) -> Self { Self { matrix } }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  fn smoke() -> impl WidgetBuilder {
    fn_widget! {
      @TransformBox {
        matrix: Transform::new(2., 0., 0., 2., 0., 0.),
        @SizedBox { size: Size::new(100., 100.) }
      }
    }
  }
  widget_layout_test!(smoke, width == 200., height == 200.,);
}
