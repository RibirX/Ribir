#![cfg(test)]
use crate::{
  prelude::*,
  widget::{layout::flex::CrossAxisAlign, Row},
};
#[derive(Clone, Default, Debug, Declare)]
pub struct EmbedPostWithKey {
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl Compose for EmbedPostWithKey {
  fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    widget! {
      track { this }
      declare Row {
        key: 0i32,
        v_align: CrossAxisAlign::Start,
        Text { text: format!("Embed{} test title", this.level), key: 1i32}
          Text { text: this.author, key: 2i32}
          Text { text: this.content, key: 3i32}
          ExprWidget {
            (this.level > 0).then(|| {
                widget! {
                  declare EmbedPostWithKey {
                    key: "embed",
                    author: this.author,
                    content: this.content,
                    level: this.leave - 1,
                  }
                }
            })
        }
      }
    }
  }
}

impl EmbedPostWithKey {
  pub fn new(level: usize) -> Self { EmbedPostWithKey { level, ..Default::default() } }
}
