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

impl<'c> ComposeChild<'c> for PaintingStyleWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    // We need to provide the text style for the children to access.
    match this.try_into_value() {
      Ok(this) => {
        let style = this.painting_style.clone();
        WrapRender::combine_child(State::value(this), child).attach_data(Box::new(Queryable(style)))
      }
      Err(this) => {
        let style = this.map_reader(|w| PartData::from_ref(&w.painting_style));
        WrapRender::combine_child(this, child).attach_data(Box::new(style))
      }
    }
  }
}

impl WrapRender for PaintingStyleWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let old = ctx.set_painting_style(self.painting_style.clone());
    let size = host.perform_layout(clamp, ctx);
    ctx.set_painting_style(old);
    size
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
