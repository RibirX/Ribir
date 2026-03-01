use wrap_render::WrapRender;

use super::*;

/// A widget that paints a background box for its host using the provided
/// brush. If a `Radius` provider is present, corners will be rounded.
///
/// This is a built-in `FatObj` field. Setting the `background` field attaches
/// a `Background` widget to the host to draw backgrounds based on layout size.
///
/// # Example
///
/// Fill the text background with a red color.
///
/// ```rust
/// use ribir::prelude::*;
///
/// text! {
///   text: "I have a red background",
///   background: Color::RED,
/// };
/// ```
#[derive(Default, Clone)]
pub struct Background {
  /// The background of the box.
  pub background: Brush,
}

impl Declare for Background {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for Background {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();

    if !size.is_empty() {
      let rect = Rect::from_size(size);
      let (provider_ctx, mut painter) = ctx.provider_ctx_and_box_painter();
      let old_brush = painter.fill_brush().clone();

      painter.set_fill_brush(self.background.clone());
      if let Some(radius) = Provider::of::<Radius>(provider_ctx) {
        painter.rect_round(&rect, &radius, true);
      } else {
        painter.rect(&rect, true);
      }
      painter.fill();

      painter.set_fill_brush(old_brush);
    }
    host.paint(ctx);
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("background") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({ "brush": self.background }))
  }
}

impl_compose_child_for_wrap_render!(Background);
