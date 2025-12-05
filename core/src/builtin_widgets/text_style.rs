use crate::prelude::*;

/// This widget establishes the text style for painting the text within its
/// descendants. This is a builtin field of FatObj. You can simply set the
/// `text_style` field to attach a TextStyleWidget to the host widget.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "Big Text",
///     text_style: TypographyTheme::of(BuildCtx::get()).title_large.text.clone(),
///   }
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
