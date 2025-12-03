use crate::prelude::*;

/// Controls alignment of multi-line text within its bounds. Descendant
/// `Text` widgets will use the `TextAlign` value to determine line alignment.
///
/// This is a built-in `FatObj` field. Setting `text_align` attaches a
/// `TextAlignWidget` that provides alignment to descendant text renderers.
///
/// # Example
///
/// Center-align all lines of a multi-line `Text` widget.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(120., 40.),
///   text_align: TextAlign::Center,
///   @Text { text: "Line 1\nlong line 2" }
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
