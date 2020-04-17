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
  fn build<'a>(&self) -> Box<dyn Widget + 'a> {
    let mut children = vec![
      Text(self.title).into(),
      Text(self.author).into(),
      Text(self.content).into(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(embed.into())
    }
    Row(children).into()
  }
}

pub fn create_embed_app<'a>(level: usize) -> Application<'a> {
  let post = EmbedPost::new(level);
  let mut app = Application::new();
  app.widget_tree.set_root(post.into(), &mut app.render_tree);
  app
}
