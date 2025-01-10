use crate::prelude::*;

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
    let provider = match this.try_into_value() {
      Ok(this) => Provider::new(this.text_style),
      Err(this) => Provider::value_of_writer(
        this.map_writer(|w| PartData::from_ref_mut(&mut w.text_style)),
        Some(DirtyPhase::LayoutSubtree),
      ),
    };

    Providers::new([provider])
      .with_child(child)
      .into_widget()
  }
}
