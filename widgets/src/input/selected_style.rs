use ribir_core::prelude::*;

#[derive(Declare)]
pub struct SelectedTextStyle {}

impl ComposeStyle for SelectedTextStyle {
  type Host = Widget;
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget
  where
    Self: Sized,
  {
    widget! {
      DynWidget {
        background: Color::from_rgb(181, 215, 254), // todo: follow application active state
        dyns: host,
      }
    }
  }
}
