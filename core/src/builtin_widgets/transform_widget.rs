use crate::{prelude::*, wrap_render::*};

#[derive(Clone, Default)]
pub struct TransformWidget {
  pub transform: Transform,
}

impl Declare for TransformWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(TransformWidget);

impl WrapRender for TransformWidget {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.painter().apply_transform(&self.transform);
    host.paint(ctx)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &HitTestCtx, pos: Point) -> HitTest {
    if let Some(t) = self.transform.inverse() {
      let pos = t.transform_point(pos);
      host.hit_test(ctx, pos)
    } else {
      HitTest { hit: false, can_hit_child: false }
    }
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    if let Some(t) = host.get_transform() {
      Some(self.transform.then(&t))
    } else {
      Some(self.transform)
    }
  }
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

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      @TransformWidget {
        transform: Transform::new(2., 0., 0., 2., 0., 0.),
        @MockBox {
          size: Size::new(100., 100.)
        }
      }
    }),
    LayoutCase::default().with_size((100., 100.).into())
  );
}
