#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl CombinationWidget for RecursiveRow {
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    Row::from_iter((0..self.width).map(|_| {
      if self.depth > 1 {
        RecursiveRow {
          width: self.width,
          depth: self.depth - 1,
        }
        .box_it()
      } else {
        Text("leaf".to_string()).box_it()
      }
    }))
    .box_it()
  }
}
