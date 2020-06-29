#![cfg(test)]
use crate::{prelude::*, widget::Row};
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl CombinationWidget for RecursiveRow {
  fn build(&self) -> Box<dyn Widget> {
    Row(
      (0..self.width)
        .map(|_| {
          if self.depth > 1 {
            RecursiveRow {
              width: self.width,
              depth: self.depth - 1,
            }
            .into()
          } else {
            Text("leaf".to_string()).into()
          }
        })
        .collect(),
    )
    .into()
  }
}
