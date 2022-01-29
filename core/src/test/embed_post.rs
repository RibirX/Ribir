#![cfg(test)]
use crate::{
  prelude::*,
  widget::layout::{flex::CrossAxisAlign, Row},
};

#[derive(Clone, Debug)]
pub struct EmbedPost {
  title: &'static str,
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl EmbedPost {
  pub fn new(level: usize) -> Self {
    EmbedPost {
      title: "Simple demo",
      author: "Adoo",
      content: "Recursive x times",
      level,
    }
  }
}

impl CombinationWidget for EmbedPost {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    declare! {
      Row {
        cross_align: CrossAxisAlign::Start,
        ..<_>::default(),
        Text { text: self.title, style: <_>::default() },
        Text { text: self.author, style: <_>::default() },
        Text { text: self.content, style: <_>::default() },
        (self.level >0).then(||{
          let mut embed = self.clone();
          embed.level -= 1;
          embed
        })
      }
    }
  }
}
