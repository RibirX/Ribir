#![cfg(test)]
use crate::prelude::*;

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
      Row {
        align_items: Align::Start,
        Text { text: this.title }
        Text { text: this.author }
        Text { text: this.content }
        ExprWidget {
          expr: (this.level > 0).then(|| EmbedPost::new(this.level - 1 ))
        }
      }
    }
  }
}
