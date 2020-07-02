use super::stateful::Stateful;
use crate::prelude::*;
use std::{cell::RefCell, rc::Rc};
pub struct BuildCtx {
  pub(crate) tree: Rc<RefCell<widget_tree::WidgetTree>>,
  current: WidgetId,
}
