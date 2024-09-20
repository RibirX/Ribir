use crate::{prelude::*, wrap_render::WrapRender};

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

impl<'c> ComposeChild<'c> for Foreground {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

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
}
