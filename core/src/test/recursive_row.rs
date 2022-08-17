#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl Compose for RecursiveRow {
  fn compose(this: StateWidget<Self>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Row {
        ExprWidget {
          expr: (0..this.width)
            .map(move |_| {
              if this.depth > 1 {
                RecursiveRow {
                  width: this.width,
                  depth: this.depth - 1,
                }
                .into_widget()
              } else {
                Text { text: "leaf".into(), style: <_>::default() }.into_widget()
              }
            })
        }
      }
    }
  }
}
