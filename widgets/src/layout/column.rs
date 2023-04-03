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
  #[declare(default)]
  pub item_gap: f32,
  #[declare(default)]
  pub line_gap: f32,
}

impl ComposeChild for Column {
  type Child = Vec<Widget>;
  fn compose_child(this: State<Self>, children: Self::Child) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      Flex {
        reverse: this.reverse,
        wrap: this.wrap,
        direction: Direction::Vertical,
        align_items: this.align_items,
        justify_content: this.justify_content,
        main_axis_gap: this.item_gap,
        cross_axis_gap: this.line_gap,
        widget::from(children)
      }
    }
  }
}
