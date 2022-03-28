#![cfg(test)]
use crate::{
  prelude::*,
  widget::{layout::flex::CrossAxisAlign, Row},
};
#[derive(Clone, Default, Debug)]
pub struct EmbedPostWithKey {
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl CombinationWidget for EmbedPostWithKey {
  #[widget]

  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      Row {
        key: 0,
        v_align: CrossAxisAlign::Start,
        Text { text: format!("Embed{} test title", self.level), key: 1},
        Text { text: self.author, key: 2},
        Text { text: self.content, key: 3},
        (self.level > 0).then(||{
          let mut embed = self.clone();
          embed.level -= 1;
          embed.with_key("embed")
        })
      }
    }
  }
}

impl EmbedPostWithKey {
  pub fn new(level: usize) -> Self { EmbedPostWithKey { level, ..Default::default() } }
}
