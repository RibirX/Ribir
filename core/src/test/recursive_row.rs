#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl CombinationWidget for RecursiveRow {
  #[widget]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare Row {
        ExprChild {
          (0..self.width)
            .map(|_| {
              if self.depth > 1 {
                RecursiveRow {
                  width: self.width,
                  depth: self.depth - 1,
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
