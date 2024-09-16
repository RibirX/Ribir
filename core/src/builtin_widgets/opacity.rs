use crate::{prelude::*, wrap_render::WrapRender};

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

impl<'c> ComposeChild<'c> for Opacity {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for Opacity {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx.painter().apply_alpha(self.opacity);
    host.paint(ctx)
  }
}
