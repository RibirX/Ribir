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
