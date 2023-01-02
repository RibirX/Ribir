use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Declare, Clone)]
pub struct TransformWidget {
  #[declare(builtin, default)]
  pub transform: Transform,
}

impl Render for TransformWidget {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_transform(&self.transform); }

  #[inline]
  fn can_overflow(&self) -> bool { true }

  #[inline]
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    let is_hit = self.transform.inverse().map_or(false, |transform| {
      hit_test_impl(ctx, transform.transform_point(pos))
    });

    HitTest { hit: is_hit, can_hit_child: is_hit }
  }

  fn get_transform(&self) -> Option<Transform> { Some(self.transform) }
}

impl Query for TransformWidget {
  impl_query_self_only!();
}

impl TransformWidget {
  #[inline]
  pub fn new(transform: Transform) -> Self { Self { transform } }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    let widget = widget! {
      TransformWidget {
        transform: Transform::new(2., 0., 0., 2., 0., 0.),
        MockBox {
          size: Size::new(100., 100.)
        }
      }
    };

    expect_layout_result(
      widget,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(Size::new(100., 100.)),
      }],
    );
  }
}
