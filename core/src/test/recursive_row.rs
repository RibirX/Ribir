#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl Compose for RecursiveRow {
  fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      track { this }
      declare Row {
        ExprWidget {
          (0..this.width)
            .map(|_| {
              if this.depth > 1 {
                RecursiveRow {
                  width: this.width,
                  depth: this.depth - 1,
                }
                .box_it()
              } else {
                Text { text: "leaf".into(), style: <_>::default() }.box_it()
              }
            })
        }
      }
    }
  }
}
