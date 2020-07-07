use crate::prelude::*;
use std::{
  cell::{Ref, RefCell},
  marker::PhantomData,
  mem::ManuallyDrop,
  rc::Rc,
};

pub struct Stateful<T> {
  wid: WidgetId,
  tree: Rc<RefCell<widget_tree::WidgetTree>>,
  // `widget` field is necessary, when modify widget state, we can just borrow
  // from widget instead of the whole tree.
  widget: Rc<RefCell<BoxWidget>>,
  _type: PhantomData<*const T>,
}
/// A mutable memory location with dynamically checked borrow rules of
/// `T` widget. Usage like `std::cell:RefCell`.
pub struct StatefulRef<T>(Stateful<T>);

/// `Stateful` erased widget type info and used only as common identify type for
/// all stateful widget.
#[derive(Debug)]
pub struct StatefulWidget {
  wid: WidgetId,
  widget: Rc<RefCell<BoxWidget>>,
}

impl Widget for StatefulWidget {
  fn classify(&self) -> WidgetClassify {
    let ptr = &*self.widget.borrow() as *const dyn Widget;
    // Safety: StatefulWidget is a inner temporary widget and not support clone,
    // it's only used when build widget tree.
    unsafe { (&*ptr).classify() }
  }
  fn classify_mut(&mut self) -> WidgetClassifyMut {
    let ptr = &mut *self.widget.borrow_mut() as *mut dyn Widget;
    // Safety: StatefulWidget is a inner temporary widget and not support clone,
    // it's only used when build widget tree.
    unsafe { (&mut *ptr).classify_mut() }
  }
}

impl StatefulWidget {
  #[inline]
  pub fn id(&self) -> WidgetId { self.wid }
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

impl<T: Widget> Stateful<T> {
  /// Return a mutable memory location with dynamically checked borrow rules of
  /// `T` widget.
  pub fn as_cell_ref(&self) -> StatefulRef<T> { StatefulRef::new(self) }

  pub(crate) fn new(tree: Rc<RefCell<widget_tree::WidgetTree>>, widget: T) -> Self {
    let widget = Rc::new(RefCell::new(widget.box_it()));
    let wid = tree.borrow_mut().new_node(widget.clone());
    Self {
      tree,
      widget,
      wid,
      _type: PhantomData,
    }
  }

  pub(crate) fn into_widget(self) -> StatefulWidget {
    StatefulWidget {
      wid: self.wid,
      widget: self.widget,
    }
  }
}

impl<T: 'static> StatefulRef<T> {
  /// Immutably borrows the wrapped value. The borrow lasts until the returned
  /// Ref exits scope. Multiple immutable borrows can be taken out at the same
  /// time.
  pub fn borrow(&self) -> Ref<T> {
    Ref::map(self.0.widget.borrow(), |w| {
      Widget::downcast_ref::<T>(w)
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
    let ref_mut = std::cell::RefMut::map(self.0.widget.borrow_mut(), |w| {
      Widget::downcast_mut::<T>(w)
        .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
    });
    RefMut {
      ref_mut: ManuallyDrop::new(ref_mut),
      tree: self.0.tree.clone(),
      wid: self.0.wid,
    }
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

pub struct RefMut<'a, T> {
  ref_mut: ManuallyDrop<std::cell::RefMut<'a, T>>,
  wid: WidgetId,
  tree: Rc<RefCell<widget_tree::WidgetTree>>,
}

impl<'a, T> Drop for RefMut<'a, T> {
  fn drop(&mut self) {
    let Self { tree, wid, ref_mut } = self;
    unsafe { ManuallyDrop::drop(ref_mut) };
    wid.mark_changed(&mut tree.borrow_mut());
  }
}

impl<'a, T> std::ops::Deref for RefMut<'a, T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.ref_mut }
}

impl<'a, T> std::ops::DerefMut for RefMut<'a, T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ref_mut }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let tree = widget_tree::WidgetTree::default();
    let tree = Rc::new(RefCell::new(tree));
    let ctx = BuildCtx { tree: tree.clone() };
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let cell_ref = {
      let t = Text("Hello".to_string());
      let (_, cell_ref) = t.into_stateful(&ctx);
      cell_ref
    };
    {
      cell_ref.borrow_mut().0 = "World!".to_string();
    }
    assert!(tree.borrow().changed_widgets.get(&cell_ref.0.wid).is_some());
  }
}
