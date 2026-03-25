use crate::prelude::*;

/// Provides text decoration information to descendant text widgets.
///
/// This is a built-in `FatObj` field. Setting `text_decoration` attaches a
/// `TextDecorationWidget` which supplies decoration lines and optional
/// decoration color to descendant text renderers.
#[derive(Default)]
pub struct TextDecorationWidget {
  pub text_decoration: TextDecorationStyle,
}

impl Declare for TextDecorationWidget {
  type Builder = FatObj<()>;

  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for TextDecorationWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    Providers::new([Self::into_provider(this)]).with_child(child)
  }
}

impl TextDecorationWidget {
  pub fn into_provider(this: impl StateWriter<Value = Self>) -> Provider {
    match this.try_into_value() {
      Ok(this) => Provider::new(this.text_decoration),
      Err(this) => Provider::writer(
        this.part_writer(PartialId::any(), |w| PartMut::new(&mut w.text_decoration)),
        Some(DirtyPhase::LayoutSubtree),
      ),
    }
  }

  pub fn inherit_widget() -> Self {
    Self {
      text_decoration: Provider::of::<TextDecorationStyle>(BuildCtx::get())
        .map(|style| (*style).clone())
        .unwrap_or_default(),
    }
  }
}
