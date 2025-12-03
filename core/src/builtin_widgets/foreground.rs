use crate::{prelude::*, wrap_render::*};

/// A widget that provides a foreground brush for painting elements in its
/// subtree. The foreground brush is inherited by descendant widgets; children
/// can access it via the `Provider`. The built-in `Text` widget uses this brush
/// when painting text.
///
/// # Example
/// Apply a foreground brush to render text in red.
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "I am red!",
///     foreground: Color::RED,
///   }
/// };
/// ```
#[derive(Default)]
pub struct Foreground {
  pub foreground: Brush,
}

impl Declare for Foreground {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(Foreground);

impl WrapRender for Foreground {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_fill_brush(self.foreground.clone())
      .set_stroke_brush(self.foreground.clone());
    host.paint(ctx)
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}
