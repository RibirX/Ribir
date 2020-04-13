#![cfg(test)]
use crate::prelude::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Default, Debug)]
struct EmbedKeyPost {
  title: Rc<RefCell<&'static str>>,
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl CombinationWidget for EmbedKeyPost {
  fn build<'a>(&self) -> Widget<'a> {
    let mut children = vec![
      KeyDetect::new(0, Text(*self.title.borrow())).to_widget(),
      KeyDetect::new(1, Text(self.author)).to_widget(),
      KeyDetect::new(2, Text(self.content)).to_widget(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(KeyDetect::new("embed", embed).to_widget())
    }
    KeyDetect::new(0, Row::new(children)).to_widget()
  }
}

pub struct KeyDetectEnv<'a> {
  pub app: Application<'a>,
  pub title: Rc<RefCell<&'static str>>,
}

impl<'a> KeyDetectEnv<'a> {
  pub fn new(level: usize) -> Self {
    let mut post = EmbedKeyPost::default();
    post.level = level;
    let title = post.title.clone();
    post.title = title.clone();

    let mut env = KeyDetectEnv {
      app: Application::default(),
      title,
    };
    let widget_tree = &mut env.app.widget_tree;
    let root = widget_tree.set_root(post.to_widget());
    widget_tree.inflate(root);
    env
  }
}
