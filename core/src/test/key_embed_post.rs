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
  fn build(&self, _: &mut BuildCtx) -> BoxWidget {
    let mut children = vec![
      Text(self.title.borrow().to_string()).with_key(0).box_it(),
      Text(self.author.to_string()).with_key(1).box_it(),
      Text(self.content.to_string()).with_key(2).box_it(),
    ];

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      children.push(embed.with_key("embed").box_it())
    }
    row(children).with_key(0).box_it()
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
    widget_tree.set_root(post.box_it(), &mut render_tree);
    KeyDetectEnv {
      widget_tree,
      render_tree,
      title,
    }
  }
}
