use wrap_render::WrapRender;

use crate::prelude::*;

/// A wrapper that clips the host widget to its boundary rectangle, optionally
/// respecting corner radius.
///
/// This is a built-in `FatObj` field. Setting `clip_boundary` attaches a
/// `ClipBoundary` to the host and will clip contents to the host box.
///
/// # Example
///
/// Clip text to the parent rect.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(40., 20.),
///   clip_boundary: true,
///   @Text { text: "long text will be clipped" }
/// };
/// ```
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

impl_compose_child_for_wrap_render!(ClipBoundary);

impl WrapRender for ClipBoundary {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    host.measure(clamp, ctx)
  }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool { host.size_affected_by_child() }

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

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let mut hit = host.hit_test(ctx, pos);

    // Clip child hit testing to box boundaries
    hit.can_hit_child &= ctx.box_hit_test(pos);

    hit
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("clipBoundary") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({ "enabled": self.clip_boundary }))
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

    let size = Size::new(40., 40.);
    assert_widget_eq_image!(
      WidgetTester::new(fn_widget! {
        @MockBox {
          clip_boundary: true,
          radius: Radius::all(20.),
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
