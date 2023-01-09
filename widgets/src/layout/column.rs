use super::{flex::*, Direction};
use ribir_core::prelude::*;

#[derive(Default, Declare, Clone)]
pub struct Column {
  #[declare(default)]
  pub reverse: bool,
  #[declare(default)]
  pub wrap: bool,
  #[declare(default)]
  pub align_items: Align,
  #[declare(default)]
  pub justify_content: JustifyContent,
}

impl ComposeChild for Column {
  type Child = Vec<Widget>;
  fn compose_child(this: State<Self>, children: Self::Child) -> Widget {
    widget_maybe_states! {
      maybe_states { this }
      Flex {
        reverse: this.reverse,
        wrap: this.wrap,
        direction: Direction::Vertical,
        align_items: this.align_items,
        justify_content: this.justify_content,
        DynWidget { dyns: children }
      }
    }
  }
}
