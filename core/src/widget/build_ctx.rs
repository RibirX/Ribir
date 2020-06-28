use super::stateful::Stateful;
use crate::prelude::*;
use std::{cell::RefCell, rc::Rc};
pub struct BuildCtx {
  tree: Rc<RefCell<widget_tree::WidgetTree>>,
  current: WidgetId,
}

impl BuildCtx {
  pub fn as_stateful<W: Into<Box<dyn Widget>> + 'static>(&mut self, widget: W) -> Stateful<W> {
    let wid = self.tree.borrow_mut().new_node(widget.into());
    Stateful::new(wid, self.tree.clone())
  }
}
