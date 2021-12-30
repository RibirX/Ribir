#![cfg(test)]
use crate::{
  prelude::*,
  render::render_tree::*,
  widget::{
    layout::{flex::CrossAxisAlign, Row},
    widget_tree::*,
  },
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
        Text { text: self.title },
        Text { text: self.author },
        Text { text: self.content },
        (self.level >0).then(||{
          let mut embed = self.clone();
          embed.level -= 1;
          embed
        })
      }
    }
  }
}

pub fn create_embed_app(level: usize) -> (WidgetTree, RenderTree) {
  let post = EmbedPost::new(level);
  let mut widget_tree = WidgetTree::default();
  let mut render_tree = RenderTree::default();

  widget_tree.set_root(post.box_it(), &mut render_tree);
  (widget_tree, render_tree)
}
