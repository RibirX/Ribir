#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl CombinationWidget for RecursiveRow {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    declare! {
      Row{
        ..<_>::default(),
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
