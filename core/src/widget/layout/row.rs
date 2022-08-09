use super::flex::*;
use crate::prelude::*;

#[derive(Default, Declare, Clone)]
pub struct Row {
  #[declare(default)]
  pub reverse: bool,
  #[declare(default)]
  pub wrap: bool,
  #[declare(default = Align::Center)]
  pub align_items: Align,
  #[declare(default)]
  pub justify_content: JustifyContent,
}

impl ComposeMultiChild for Row {
  fn compose_multi_child(this: Stateful<Self>, children: Vec<Widget>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this }
      Flex {
        reverse: this.reverse,
        wrap: this.wrap,
        direction: Direction::Horizontal,
        align_items: this.align_items,
        justify_content: this.justify_content,
        ExprWidget { expr: children.into_iter() }
      }
    }
  }
}
