use ribir_core::prelude::*;

use crate::layout::{Container, Stack};

use super::InputStyle;

#[derive(Declare)]
pub(crate) struct SelectedText {
  pub(crate) rects: Vec<Rect>,
}

impl Compose for SelectedText {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let color = InputStyle::of(ctx).select_background.clone();
      }
      Stack {
        DynWidget {
          dyns: {
            this.rects.iter().copied()
            .map(|rc| {
              let color = color.clone();
              widget! {
                  Container {
                    background: color,
                    top_anchor: rc.origin.y,
                    left_anchor: rc.origin.x,
                    size: rc.size,
                  }
              }.into_widget()
            }).collect::<Vec<_>>()
          }
        }
      }
    }
    .into_widget()
  }
}
