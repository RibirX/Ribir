use crate::{prelude::*, wrap_render::WrapRender};

/// This widget establishes the text style for painting the text within its
/// descendants.
#[derive(Default)]
pub struct TextStyleWidget {
  pub text_style: TextStyle,
}

impl Declare for TextStyleWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for TextStyleWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    // We need to provide the text style for the children to access.
    match this.try_into_value() {
      Ok(this) => {
        let style = this.text_style.clone();
        WrapRender::combine_child(State::value(this), child).attach_data(Box::new(Queryable(style)))
      }
      Err(this) => {
        let style = this.map_reader(|w| PartData::from_ref(&w.text_style));
        WrapRender::combine_child(this, child).attach_data(Box::new(style))
      }
    }
  }
}

impl WrapRender for TextStyleWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_text_style(self.text_style.clone());

    host.paint(ctx)
  }
}
