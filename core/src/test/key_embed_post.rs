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
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    declare! {
      Row{
        key: 0,
        cross_align: CrossAxisAlign::Start,
        ..<_>::default(),
        Text { text: format!("Embed{} test title", self.level) , style: <_>::default(), key: 1},
        Text { text: self.author, style: <_>::default(), key: 2},
        Text { text: self.content, style: <_>::default(), key: 3},
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
