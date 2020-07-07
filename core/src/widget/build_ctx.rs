use crate::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct BuildCtx {
  pub(crate) tree: Rc<RefCell<widget_tree::WidgetTree>>,
}

impl BuildCtx {
  #[inline]
  pub(crate) fn new(tree: Rc<RefCell<widget_tree::WidgetTree>>, current: WidgetId) -> Self {
    Self { tree }
  }
}
