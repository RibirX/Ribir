use crate::{prelude::*, wrap_render::*};

/// A wrapper that controls the opacity of its child.
///
/// This is a built-in `FatObj` field. Setting the `opacity` field attaches an
/// `Opacity` wrapper which applies an alpha multiplier when painting.
///
/// # Example
///
/// Display a red container with 50% opacity.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   background: Color::RED,
///   opacity: 0.5,
/// };
/// ```
#[derive(Clone)]
pub struct Opacity {
  pub opacity: f32,
}

impl Declare for Opacity {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Default for Opacity {
  #[inline]
  fn default() -> Self { Self { opacity: 1.0 } }
}

impl_compose_child_for_wrap_render!(Opacity);

impl WrapRender for Opacity {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.painter().apply_alpha(self.opacity);
    if self.opacity > 0. {
      host.paint(ctx)
    }
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    if self.opacity > 0. {
      host.visual_box(ctx)
    } else {
      ctx.clip(Rect::from_size(Size::zero()));
      None
    }
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("opacity") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({ "alpha": self.opacity }))
  }
}
