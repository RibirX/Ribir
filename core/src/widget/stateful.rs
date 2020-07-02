use crate::prelude::*;
use std::{
  cell::{Ref, RefCell, RefMut},
  marker::PhantomData,
  rc::Rc,
};

pub struct Stateful<T> {
  wid: WidgetId,
  tree: Rc<RefCell<widget_tree::WidgetTree>>,
  // `widget` field is necessary, when modify widget state, we can just borrow
  // from widget instead of the whole tree.
  widget: Rc<RefCell<Box<dyn Widget>>>,
  _type: PhantomData<*const T>,
}
/// A mutable memory location with dynamically checked borrow rules of
/// `T` widget. Usage like `std::cell:RefCell`.
pub struct StatefulRef<T>(Stateful<T>);

/// `Stateful` erased widget type info and used only as common identify type for
/// all stateful widget.
#[derive(Debug)]
struct StatefulWidget {
  wid: WidgetId,
  widget: Rc<RefCell<Box<dyn Widget>>>,
}

impl Widget for StatefulWidget {
  fn classify(&self) -> WidgetClassify {
    let ptr = &**self.widget.borrow() as *const dyn Widget;
    // Safety: StatefulWidget is a inner widget and not support clone, it's only
    // used when build widget tree.
    unsafe { (&*ptr).classify() }
  }
  fn classify_mut(&mut self) -> WidgetClassifyMut {
    let ptr = &mut **self.widget.borrow_mut() as *mut dyn Widget;
    // Safety: StatefulWidget is a inner widget and not support clone, it's only
    // used when build widget tree.
    unsafe { (&mut *ptr).classify_mut() }
  }
}

impl<T> From<Stateful<T>> for Box<dyn Widget> {
  fn from(s: Stateful<T>) -> Self {
    Box::new(StatefulWidget {
      wid: s.wid,
      widget: s.widget,
    })
  }
}

impl<T: 'static> Clone for StatefulRef<T> {
  #[inline]
  fn clone(&self) -> Self { Self::new(&self.0) }
}

impl<T> Drop for StatefulRef<T> {
  fn drop(&mut self) {
    let Stateful { tree, wid, .. } = &self.0;
    let mut tree = tree.borrow_mut();
    if let Some(WidgetClassify::Combination(_)) = wid.get(&tree).map(|w| w.classify()) {
      wid.mark_needs_build(&mut tree);
    } else {
      wid.mark_changed(&mut tree);
    }
  }
}
impl<T: 'static> Stateful<T> {
  /// Return a mutable memory location with dynamically checked borrow rules of
  /// `T` widget.
  pub fn as_cell_ref(&self) -> StatefulRef<T> { StatefulRef::new(self) }

  pub(crate) fn new(tree: Rc<RefCell<widget_tree::WidgetTree>>, widget: T) -> Self {
    unimplemented!();
  }
}

impl<T: 'static> StatefulRef<T> {
  /// Immutably borrows the wrapped value. The borrow lasts until the returned
  /// Ref exits scope. Multiple immutable borrows can be taken out at the same
  /// time.
  pub fn borrow(&self) -> Ref<T> {
    Ref::map(self.0.widget.borrow(), |w| {
      w.downcast_ref::<T>()
        .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
    })
  }

  /// Mutably borrows the wrapped value and mark the widget as dirty.
  /// The borrow lasts until the returned RefMut or all RefMuts derived from it
  /// exit scope. The value cannot be borrowed while this borrow is active.

  /// Remember framework assume you will change the wrapped widget 's state
  /// after called `borrow_mut`.
  /// Panics if the value is currently borrowed. For a non-panicking variant,
  /// use try_borrow_mut.
  pub fn borrow_mut(&self) -> RefMut<T> {
    RefMut::map(self.0.widget.borrow_mut(), |w| {
      w.downcast_mut::<T>()
        .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
    })
  }

  fn new(stateful: &Stateful<T>) -> Self {
    StatefulRef(Stateful {
      widget: stateful.widget.clone(),
      tree: stateful.tree.clone(),
      wid: stateful.wid,
      _type: stateful._type,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let tree = widget_tree::WidgetTree::default();
    let tree = Rc::new(RefCell::new(tree));
    let root = tree.borrow_mut().new_node(Text("root".to_string()).into());
    let ctx = BuildCtx {
      tree: tree.clone(),
      current: root,
    };

    // Simulate text widget is used as other widget' s child, and want modify widget
    // in other place. So return a cell ref of the `Text` but not own it. Can use
    // the `cell_ref` in some closure.
    let cell_ref = {
      let t = Text("Hello".to_string());
      let stateful = t.into_stateful(&ctx);
      stateful.as_cell_ref()
    };
    {
      cell_ref.borrow_mut().0 = "World!".to_string();
    }
    assert!(tree.borrow().changed_widgets.get(&cell_ref.0.wid).is_some());
  }
}
