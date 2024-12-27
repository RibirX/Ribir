use crate::{prelude::*, wrap_render::WrapRender};

/// This widget establishes the text style for painting the text within its
/// descendants.
pub struct TextStyleWidget {
  pub text_style: TextStyleOptional,
}

impl Declare for TextStyleWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for TextStyleWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl WrapRender for TextStyleWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let old = ctx.set_text_style(self.text_style.merge(ctx.text_style()));
    let size = host.perform_layout(clamp, ctx);
    ctx.set_text_style(old);
    size
  }
}
