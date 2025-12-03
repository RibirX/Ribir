use crate::prelude::*;

/// Provides a `TextStyle` to descendant text widgets for painting text.
///
/// This is a built-in `FatObj` field. Setting `text_style` attaches a
/// `TextStyleWidget` which supplies text style information to descendants.
///
/// # Example
///
/// Paint text using the title-large style from the typography theme.
///
/// ```rust
/// use ribir::prelude::*;
///
/// text! {
///   text: "Big Text",
///   text_style: TypographyTheme::of(BuildCtx::get()).title_large.text.clone(),
/// };
/// ```
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
    Providers::new([Self::into_provider(this)]).with_child(child)
  }
}

impl TextStyleWidget {
  pub fn into_provider(this: impl StateWriter<Value = Self>) -> Provider {
    match this.try_into_value() {
      Ok(this) => Provider::new(this.text_style),
      Err(this) => Provider::writer(
        this.part_writer(PartialId::any(), |w| PartMut::new(&mut w.text_style)),
        Some(DirtyPhase::LayoutSubtree),
      ),
    }
  }
}

impl TextStyleWidget {
  pub fn inherit_widget() -> Self {
    TextStyleWidget {
      text_style: Provider::of::<TextStyle>(BuildCtx::get())
        .unwrap()
        .clone(),
    }
  }
}
