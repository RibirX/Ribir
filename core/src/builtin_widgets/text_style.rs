use crate::{prelude::*, wrap_render::WrapRender};

/// This widget establishes the text style for painting the text within its
/// descendants.
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
    let (child, provider): (_, Box<dyn Query>) = match this.try_into_value() {
      Ok(this) => {
        let style = this.text_style.clone();
        (WrapRender::combine_child(State::value(this), child), Box::new(Queryable(style)))
      }
      Err(this) => {
        let style = this.map_reader(|w| PartData::from_ref(&w.text_style));
        (WrapRender::combine_child(this, child), Box::new(style))
      }
    };

    let ctx = BuildCtx::get_mut();
    ctx.current_providers.push(provider);
    child.into_widget().on_build(|id| {
      let provider = ctx.current_providers.pop().unwrap();
      id.attach_data(provider, ctx.tree_mut());
    })
  }
}

impl WrapRender for TextStyleWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let old = ctx.set_text_style(self.text_style.clone());
    let size = host.perform_layout(clamp, ctx);
    ctx.set_text_style(old);
    size
  }
}
