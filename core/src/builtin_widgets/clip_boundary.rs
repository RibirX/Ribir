use wrap_render::WrapRender;

use crate::prelude::*;

/// This widget use to clip the host widget by the boundary rect with radius.
#[derive(Default, Clone)]
pub struct ClipBoundary {
  /// If true, clip the host widget by the boundary rect with radius, else do
  /// nothing.
  pub clip_boundary: bool,
}

impl Declare for ClipBoundary {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(ClipBoundary, DirtyPhase::Layout);

impl WrapRender for ClipBoundary {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn only_sized_by_parent(&self, host: &dyn Render) -> bool { host.only_sized_by_parent() }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.clip_boundary {
      let rect = Rect::from_size(
        ctx
          .box_size()
          .expect("impossible without size in painting stage"),
      );

      let path = if let Some(radius) = Provider::of::<Radius>(ctx) {
        Path::rect_round(&rect, &radius)
      } else {
        Path::rect(&rect)
      };

      ctx.box_painter().clip(path.into());
    }
    host.paint(ctx)
  }

  fn visual_box(&self, _: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    let clip_rect = Rect::from_size(ctx.box_size()?);
    ctx.clip(clip_rect);
    Some(clip_rect)
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  #[cfg(not(target_arch = "wasm32"))]
  fn clip_boundary() {
    reset_test_env!();

    let size = Size::new(80., 20.);
    assert_widget_eq_image!(
      WidgetTester::new(fn_widget! {
        @MockBox {
          clip_boundary: true,
          radius: Radius::all(10.),
          size: size,
          @MockBox {
            background: Color::GRAY,
            size: size,
          }
        }
      })
      .with_wnd_size(size)
      .with_comparison(0.00015),
      "clip_boundary"
    );
  }
}
