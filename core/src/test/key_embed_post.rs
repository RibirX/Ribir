#![cfg(test)]
use crate::{
  prelude::*,
  render::render_tree::*,
  widget::{row, widget_tree::*},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Default, Debug)]
struct EmbedKeyPost {
  title: Rc<RefCell<&'static str>>,
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl CombinationWidget for EmbedKeyPost {
  fn build(&self) -> Box<dyn Widget> {
    let mut children = vec![
      KeyDetect::new(0, Text(self.title.borrow().to_string())).into(),
      KeyDetect::new(1, Text(self.author.to_string())).into(),
      KeyDetect::new(2, Text(self.content.to_string())).into(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(KeyDetect::new("embed", embed).into())
    }
    KeyDetect::new(0, row(children)).into()
  }
}

pub struct KeyDetectEnv {
  pub widget_tree: WidgetTree,
  pub render_tree: RenderTree,
  pub title: Rc<RefCell<&'static str>>,
}

impl KeyDetectEnv {
  pub fn new(level: usize) -> Self {
    let mut post = EmbedKeyPost::default();
    post.level = level;
    let title = post.title.clone();
    post.title = title.clone();

    let mut widget_tree = WidgetTree::default();
    let mut render_tree = RenderTree::default();
    widget_tree.set_root(post.into(), &mut render_tree);
    KeyDetectEnv {
      widget_tree,
      render_tree,
      title,
    }
  }
}
