use ribir_core::prelude::*;

#[derive(SingleChild, Declare, Clone)]
pub struct TransformBox {
  pub matrix: Transform,
}

impl Render for TransformBox {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    self
      .matrix
      .inverse()
      .map_or_else(Size::zero, |t| {
        let min_box = t.outer_transformed_box(&Box2D::from_size(clamp.min));
        let min = min_box.size();

        let max_box = t.outer_transformed_box(&Box2D::from_size(clamp.max));
        let max = max_box.size();

        let child_clamp = BoxClamp { min, max };

        let size = ctx.assert_perform_single_child_layout(child_clamp);
        let rect = self
          .matrix
          .outer_transformed_rect(&Rect::from_size(size));
        rect.size
      })
  }

  #[inline]
  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    Some(Rect::from_size(ctx.box_size()?))
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_transform(&self.matrix); }
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

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @TransformBox {
        matrix: Transform::new(2., 0., 0., 2., 0., 0.),
        @Container { size: Size::new(100., 100.) }
      }
    }),
    LayoutCase::default().with_size(Size::new(200., 200.))
  );
}
