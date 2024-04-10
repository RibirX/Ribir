use crate::prelude::*;

#[derive(SingleChild, Query, Clone, Default)]
pub struct TransformWidget {
  pub transform: Transform,
}

impl Declare for TransformWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Render for TransformWidget {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_transform(&self.transform); }

  #[inline]
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    let is_hit = self
      .transform
      .inverse()
      .map_or(false, |transform| hit_test_impl(ctx, transform.transform_point(pos)));

    HitTest { hit: is_hit, can_hit_child: is_hit }
  }

  fn get_transform(&self) -> Option<Transform> { Some(self.transform) }
}

impl TransformWidget {
  #[inline]
  pub fn new(transform: Transform) -> Self { Self { transform } }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  fn smoke() -> impl WidgetBuilder {
    fn_widget! {
      @TransformWidget {
        transform: Transform::new(2., 0., 0., 2., 0., 0.),
        @MockBox {
          size: Size::new(100., 100.)
        }
      }
    }
  }
  widget_layout_test!(smoke, width == 100., height == 100.,);
}
