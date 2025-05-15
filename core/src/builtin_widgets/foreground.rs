use crate::{prelude::*, wrap_render::*};

/// A widget that sets the brush for foreground elements. It's can be inherited
/// by its descendants. When meet a color of `background`, the foreground will
/// be overwrite by it.

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
