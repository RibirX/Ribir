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

impl Compose for EmbedPost {
  #[widget]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      declare Row {
        v_align: CrossAxisAlign::Start,
        Text { text: self.title }
        Text { text: self.author }
        Text { text: self.content }
        ExprChild {
          (self.level >0).then(||{
            let mut embed = self.clone();
            embed.level -= 1;
            embed
          })
        }
      }
    }
  }
}
