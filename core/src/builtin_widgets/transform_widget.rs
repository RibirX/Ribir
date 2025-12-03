use crate::{prelude::*, wrap_render::*};

/// TransformWidget is a widget that applies a transformation to its child.
///
/// This is a builtin field of FatObj. You can simply set the `transform`
/// field to attach a TransformWidget to the host widget.
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

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    host
      .visual_box(ctx)
      .map_or(Some(Rect::from_size(ctx.box_size()?)), |rect| {
        Some(self.transform.outer_transformed_rect(&rect))
      })
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.painter().apply_transform(&self.transform);
    host.paint(ctx)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    if let Some(t) = self.transform.inverse() {
      let lt = ctx.box_pos().unwrap();
      let pos = (pos - lt).to_point();
      let pos = t.transform_point(pos) + lt.to_vector();
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

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
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
