use crate::prelude::*;

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
    let provider = match this.try_into_value() {
      Ok(this) => Provider::new(this.painting_style),
      Err(this) => Provider::value_of_writer(
        this.map_writer(|w| PartData::from_ref_mut(&mut w.painting_style)),
        Some(DirtyPhase::LayoutSubtree),
      ),
    };

    Providers::new([provider])
      .with_child(child)
      .into_widget()
  }
}
