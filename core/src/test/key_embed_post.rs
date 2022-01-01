#![cfg(test)]
use crate::{
  prelude::*,
  render::render_tree::*,
  widget::{layout::flex::CrossAxisAlign, widget_tree::*, Row},
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
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    declare! {
      Row{
        key: 0,
        cross_align: CrossAxisAlign::Start,
        ..<_>::default(),
        Text { text: self.title.borrow().to_string(), style: <_>::default(), key: 1},
        Text { text: self.author, style: <_>::default(), key: 2},
        Text { text: self.content, style: <_>::default(), key: 3},
        (self.level > 0).then(||{
          let mut embed = self.clone();
          embed.level -= 1;
          embed.with_key("embed")
        })
      }
    }
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
