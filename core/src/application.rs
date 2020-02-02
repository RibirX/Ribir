use crate::{render_object::RenderObject, widget::Widget};
use ::herald::prelude::*;
use std::collections::LinkedList;

pub struct Application<'a, W> {
  data: W,
  widget_tree: LinkedList<Box<dyn Widget + 'a>>,
  // render_tree: Box<dyn RenderObject>,
}

impl<'a, D: Widget + Herald<'a> + 'a> Application<'a, D> {
  pub fn new(data: D) -> Application<'a, D> {
    let mut widget_tree = LinkedList::new();
    if let Some(head) = data.build() {
      widget_tree.push_back(head);
    }
    Application {
      data: data,
      widget_tree,
    }
  }

  pub fn run(self) {
    let Application {
      mut data,
      widget_tree,
    } = self;
    // subscribe the data batched changes
    data.herald().batched_changes_events().subscribe_change(
      move |mut changes: ChangeEvent<'_, D, _>| {
        let root = changes.host().build();
        unimplemented!("diff widget tree and update render tree");
      },
    );
  }
}
