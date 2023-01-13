use ribir_core::prelude::*;

use crate::layout::{Container, Stack};

use super::InputTheme;

#[derive(Declare)]
pub(crate) struct SelectedText {
  pub(crate) rects: Vec<Rect>,
}

impl Compose for SelectedText {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      init ctx => {
        let color = InputTheme::of(ctx).select_background.clone();
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
              }
            }).collect::<Vec<_>>()
          }
        }
      }
    }
  }
}
