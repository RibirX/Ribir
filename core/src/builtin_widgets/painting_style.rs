use crate::{prelude::*, wrap_render::*};

/// A widget that sets the strategies for painting shapes and paths . It's can
/// be inherited by its descendants.
#[derive(Default)]
pub struct PaintingStyleWidget {
  pub painting_style: PaintingStyle,
}

impl Declare for PaintingStyleWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(PaintingStyleWidget);

impl WrapRender for PaintingStyleWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    match &self.painting_style {
      PaintingStyle::Fill => ctx.painter().set_style(PathStyle::Fill),
      PaintingStyle::Stroke(stroke_options) => ctx
        .painter()
        .set_strokes(stroke_options.clone())
        .set_style(PathStyle::Stroke),
    };

    host.paint(ctx)
  }
}
