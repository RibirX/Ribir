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
      author: "Ribir",
      content: "Recursive x times",
      level,
    }
  }
}

impl Compose for EmbedPost {
  fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this }
      declare Row {
        v_align: CrossAxisAlign::Start,
        Text { text: this.title }
        Text { text: this.author }
        Text { text: this.content }
        ExprWidget {
          expr: (this.level > 0).then(move || EmbedPost::new(this.level - 1 ))
        }
      }
    }
  }
}
