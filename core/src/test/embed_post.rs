#![cfg(test)]
use crate::{prelude::*, render::render_tree::*, widget::widget_tree::*};

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
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    let mut children = vec![
      Text(self.title.to_string()).box_it(),
      Text(self.author.to_string()).box_it(),
      Text(self.content.to_string()).box_it(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(embed.box_it())
    }
    row(children).box_it()
  }
}

pub fn create_embed_app(level: usize) -> (WidgetTree, RenderTree) {
  let post = EmbedPost::new(level);
  let mut widget_tree = WidgetTree::default();
  let mut render_tree = RenderTree::default();

  widget_tree.set_root(post.box_it(), &mut render_tree);
  (widget_tree, render_tree)
}
