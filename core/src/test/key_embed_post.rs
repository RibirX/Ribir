#![cfg(test)]
use crate::{
  prelude::*,
  render::render_tree::*,
  widget::{layout::flex::CrossAxisAlign, widget_tree::*, Row},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Default, Debug, Widget)]
struct EmbedKeyPost {
  title: Rc<RefCell<&'static str>>,
  author: &'static str,
  content: &'static str,
  level: usize,
}

impl CombinationWidget for EmbedKeyPost {
  fn build(&self, _: &mut BuildCtx) -> Box<dyn Widget> {
    let mut row = Row::default()
      .with_cross_align(CrossAxisAlign::Start)
      .push(Text(self.title.borrow().to_string()).with_key(0))
      .push(Text(self.author.to_string()).with_key(1))
      .push(Text(self.content.to_string()).with_key(2));

    if self.level > 0 {
      let mut embed = self.clone();
      embed.level -= 1;
      row = row.push(embed.with_key("embed"));
    }

    row.with_key(0).box_it()
  }
}

pub struct KeyDetectEnv {
  pub widget_tree: WidgetTree,
  pub render_tree: RenderTree,
  pub title: Rc<RefCell<&'static str>>,
}

impl KeyDetectEnv {
  pub fn new(level: usize) -> Self {
    let mut post = EmbedKeyPost { level, ..Default::default() };
    post.level = level;
    let title = post.title.clone();
    post.title = title.clone();

    let mut widget_tree = WidgetTree::default();
    let mut render_tree = RenderTree::default();
    widget_tree.set_root(post.box_it(), &mut render_tree);
    KeyDetectEnv { widget_tree, render_tree, title }
  }
}
