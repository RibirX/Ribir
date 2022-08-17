#![cfg(test)]
use crate::prelude::*;
#[derive(Clone, Default, Debug, Declare)]
pub struct EmbedPostWithKey {
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl Compose for EmbedPostWithKey {
  fn compose(this: StateWidget<Self>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Row {
        key: 0i32,
        align_items: Align::Start,
        Text {
          text: {
            let level = this.level;
            format!("Embed{} test title", level)
          },
          key: 1i32
        }
        Text { text: this.author, key: 2i32}
        Text { text: this.content, key: 3i32}
        ExprWidget {
          expr:(this.level > 0).then(move || {
              widget! {
                EmbedPostWithKey {
                  key: "embed",
                  author: this.author,
                  content: this.content,
                  level: this.level - 1,
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
