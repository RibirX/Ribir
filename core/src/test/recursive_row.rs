#![cfg(test)]
use crate::prelude::*;
#[derive(Debug)]
pub struct RecursiveRow {
  pub width: usize,
  pub depth: usize,
}

impl CombinationWidget for RecursiveRow {
  fn build<'a>(&self) -> Widget<'a> {
    Row(
      (0..self.width)
        .into_iter()
        .map(|_| {
          if self.depth > 1 {
            RecursiveRow {
              width: self.width,
              depth: self.depth - 1,
            }
            .to_widget()
          } else {
            Text("leaf").to_widget()
          }
        })
        .collect(),
    )
    .to_widget()
  }
}
