use crate::prelude::*;
use std::{
  cell::{Ref, RefCell, RefMut},
  marker::PhantomData,
  rc::Rc,
};

#[derive(Clone)]
pub struct Stateful<T> {
  wid: WidgetId,
  w_tree: Rc<RefCell<widget_tree::WidgetTree>>,
  _type: PhantomData<*const T>,
}

impl<T: 'static> Stateful<T> {
  pub fn new(wid: WidgetId, tree: Rc<RefCell<widget_tree::WidgetTree>>) -> Self {
    Self {
      wid,
      w_tree: tree,
      _type: PhantomData,
    }
  }

  pub fn borrow(&self) -> Ref<T> {
    Ref::map(self.w_tree.borrow(), |tree| {
      self
        .wid
        .get(tree)
        .and_then(|w| w.as_any().downcast_ref::<T>())
        .unwrap_or_else(|| {
          unreachable!(
            "Something wrong, Maybe use a destroyed widget id and id assigned to another widget"
          )
        })
    })
  }

  pub fn borrow_mut(&mut self) -> RefMut<T> {
    let mut tree = self.w_tree.borrow_mut();
    if let Some(WidgetClassify::Combination(_)) = self.wid.get(&tree).map(|w| w.classify()) {
      self.wid.mark_needs_build(&mut tree);
    } else {
      self.wid.mark_changed(&mut tree);
    }
    RefMut::map(tree, |tree| {
      self
        .wid
        .get_mut(tree)
        .and_then(|w| w.as_any_mut().downcast_mut::<T>())
        .unwrap_or_else(|| {
          unreachable!(
            "Something wrong, Maybe use a destroyed widget id and id assigned to another widget"
          )
        })
    })
  }
}

impl<T> From<Stateful<T>> for WidgetNode {
  fn from(state: T) -> Self { widget_tree::WidgetNode::ID(state.wid) }
}
