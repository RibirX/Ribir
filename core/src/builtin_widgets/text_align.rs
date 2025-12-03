use crate::prelude::*;

/// `TextAlign` is used to align multiline text within the text bounds. The
/// descendent `Text` widget will be aligned according to the `TextAlign` value.
///
/// This is a builtin field of FatObj. You can simply set the `text_align`
/// field to attach a TextAlign widget to the host widget.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Container {
///     size: Size::new(120., 40.),
///     text_align: TextAlign::Center,
///     @Text {
///        text: "Line 1\nLine 2",
///     }
///   }
/// };
/// ```
#[derive(Default)]
pub struct TextAlignWidget {
  pub text_align: TextAlign,
}

impl Declare for TextAlignWidget {
  type Builder = FatObj<()>;

  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for TextAlignWidget {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    Providers::new([Self::into_provider(this)]).with_child(child)
  }
}

impl TextAlignWidget {
  pub fn into_provider(this: impl StateWriter<Value = Self>) -> Provider {
    match this.try_into_value() {
      Ok(this) => Provider::new(this.text_align),
      Err(this) => Provider::writer(
        this.part_writer(PartialId::any(), |w| PartMut::new(&mut w.text_align)),
        Some(DirtyPhase::LayoutSubtree),
      ),
    }
  }
}
