#![cfg(test)]
use crate::prelude::*;
#[derive(Clone, Default, Debug, Declare)]
pub struct EmbedPostWithKey {
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl Compose for EmbedPostWithKey {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Row {
        align_items: Align::Start,
        Text {
          text: {
            let level = this.level;
            format!("Embed{} test title", level)
          },
        }
        Text { text: this.author}
        Text { text: this.content }
        ExprWidget {
          expr:(this.level > 0).then(move || {
              widget! {
                EmbedPostWithKey {
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
