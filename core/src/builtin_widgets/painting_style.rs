use crate::{prelude::*, wrap_render::WrapRender};

/// Explain the method for rendering shapes and paths, including filling or
/// stroking them.
#[derive(Clone, Debug, Default)]
pub enum PaintingStyle {
  /// Fill the path.
  #[default]
  Fill,
  /// Stroke path with line width.
  Stroke(StrokeOptions),
}

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

impl<'c> ComposeChild<'c> for PaintingStyleWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

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
