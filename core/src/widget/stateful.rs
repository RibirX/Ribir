use crate::prelude::*;
use std::{
  cell::{Ref, RefCell, RefMut},
  marker::PhantomData,
  rc::Rc,
};

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back of this reference if you
/// mutably dereference this pointer. No matter if you really modify it.
///
/// ## Panics
///
/// `StateRef` should not live longer than its widget. Framework guarantee the
/// widgets constructed in the same `build` method  have same lifetime,  and
/// parent live longer than parent. So not pass a `StateRef` to its ancestors,
/// that maybe panic.
pub struct StateRef<T: Widget> {
  pointer: StatefulPointer,
  type_info: PhantomData<*const T>,
}

/// This widget convert a stateless widget to stateful.
pub type Stateful<T> = WidgetAttr<T, StatefulAttr>;

#[derive(Debug)]
pub struct StatefulAttr(StatefulPointer);

type StatefulPointer = Rc<RefCell<(*const dyn Widget, *mut dyn Widget)>>;

impl<W: Widget> Stateful<W> {
  #[inline]
  pub fn get_state_ref(&self) -> StateRef<W> {
    StateRef {
      pointer: self.attr.0.clone(),
      type_info: PhantomData,
    }
  }
}

impl<W: Widget> StateRef<W> {
  pub fn borrow(&self) -> Ref<W> {
    Ref::map(self.pointer.borrow(), |(p, _)| unsafe {
      &*(*p as *const W)
    })
  }

  pub fn borrow_mut(&mut self) -> RefMut<W> {
    RefMut::map(self.pointer.borrow_mut(), |(_, p)| unsafe {
      &mut *(*p as *mut W)
    })
  }
}

impl StatefulAttr {
  pub fn new(widget: &mut BoxWidget) -> Self {
    let pointer = Rc::new(RefCell::new((
      &*widget.widget as *const _,
      &mut *widget.widget as *mut _,
    )));
    Self(pointer)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let stateful = Text("Hello".to_string()).into_stateful();
    {
      stateful.get_state_ref().borrow_mut().0 = "World!".to_string();
    }
    assert_eq!(&stateful.0, "World!");
  }

  #[test]
  fn inherit_from_stateful() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Text("Hello".to_string()).into_stateful();
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id
      .get(tree)
      .and_then(|w| w.downcast_attr_widget::<Key>())
      .map(|k| k.key());
    assert!(key_back.is_some());
  }

  #[test]
  fn access_state() {
    let stateful = Text("hello".to_string()).into_stateful();

    assert_eq!(stateful.0, "hello")
  }
}
