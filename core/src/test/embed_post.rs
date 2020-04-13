#![cfg(test)]
use crate::prelude::*;
use crate::widget::Row;

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
  fn build<'a>(&self) -> Widget<'a> {
    let mut children = vec![
      Text(self.title).to_widget(),
      Text(self.author).to_widget(),
      Text(self.content).to_widget(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(embed.to_widget())
    }
    Row::new(children).to_widget()
  }
}

pub fn create_embed_app<'a>(level: usize) -> Application<'a> {
  let post = EmbedPost::new(level);
  let mut app = Application::new();
  let root = app.widget_tree.set_root(post.to_widget());
  app.widget_tree.inflate(root);
  app.construct_render_tree(app.widget_tree.root().expect("must exists"));
  app
}
