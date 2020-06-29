#![cfg(test)]
use crate::{
  prelude::*, render::render_tree::*, widget::layout::row_col_layout::RowColumn,
  widget::widget_tree::*,
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
  fn build(&self) -> Box<dyn Widget> {
    let mut children = vec![
      Text(self.title.to_string()).into(),
      Text(self.author.to_string()).into(),
      Text(self.content.to_string()).into(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(embed.into())
    }
    Row(children).into()
  }
}

pub fn create_embed_app(level: usize) -> (WidgetTree, RenderTree) {
  let post = EmbedPost::new(level);
  let mut widget_tree = WidgetTree::default();
  let mut render_tree = RenderTree::default();

  widget_tree.set_root(post.into(), &mut render_tree);
  (widget_tree, render_tree)
}
