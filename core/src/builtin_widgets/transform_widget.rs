use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Declare, Clone)]
pub struct TransformWidget {
  #[declare(builtin, default)]
  pub transform: Transform,
}

impl Render for TransformWidget {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp))
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_transform(&self.transform); }

  #[inline]
  fn can_overflow(&self) -> bool { true }

  #[inline]
  fn hit_test(&self, ctx: &TreeCtx, pos: Point) -> HitTest {
    let is_hit = self.transform.inverse().map_or(false, |transform| {
      hit_test_impl(ctx, transform.transform_point(pos))
    });

    HitTest { hit: is_hit, can_hit_child: is_hit }
  }
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

    let (rect, _) = widget_and_its_children_box_rect(widget.into_widget(), Size::new(800., 800.));

    assert_eq!(rect, Rect::from_size(Size::new(100., 100.)));
  }
}
