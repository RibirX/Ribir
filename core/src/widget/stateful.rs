use crate::prelude::*;
use std::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
  pin::Pin,
  ptr::NonNull,
};

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back of this reference if you
/// mutably dereference this pointer. No matter if you really modify it.
///
/// ## Safety
/// Because `StateRef` can only be constructed in `Combination::build` method,
/// the only way to live longer than `build` method scope it capture by some
/// widget that construct in the same scope. And framework guarantee the widgets
/// constructed in the same `build` method  have same lifetime. so the wid is
/// always valid in its lifetime.
///
/// Maybe panic if widget impl the `Drop` trait, and access `StateRef` in its
/// `drop` method,  the captured `StateRef` maybe is dangling.
pub struct StateRef<T: Widget> {
  inner: InnerStateful<T>,
}

/// Hold the preallocated widget.
///
/// ## Safety
/// In safe code, `StatefulWidget` can only be constructed in
/// `Combination::build` method. And it design as a temporary widget only to
/// identify the preallocated real widget. It's only live during framework
/// construct widget tree, and framework guarantee the real widget live longer
/// than it.
#[derive(Debug)]
pub struct Stateful<T: Widget> {
  inner: InnerStateful<T>,
}

#[derive(Debug)]
struct InnerStateful<T: Widget> {
  wid: WidgetId,
  tree_ptr: NonNull<widget_tree::WidgetTree>,
  marker: PhantomData<*const T>,
}

impl<T: Widget> InheritWidget for Stateful<T> {
  #[inline]
  fn base_widget(&self) -> &dyn Widget {
    let inner = &self.inner;
    inner.wid.assert_get(unsafe { inner.tree_ptr.as_ref() })
  }

  #[inline]
  fn base_widget_mut(&mut self) -> &mut dyn Widget {
    let inner = &mut self.inner;
    inner.wid.assert_get_mut(unsafe { inner.tree_ptr.as_mut() })
  }
}

impl<T: Widget> Widget for Stateful<T> {
  #[inline]
  fn classify(&self) -> WidgetClassify { self.base_widget().classify() }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { self.base_widget_mut().classify_mut() }

  #[inline]
  fn as_inherit(&self) -> Option<&dyn InheritWidget> { Some(self) }

  #[inline]
  fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { Some(self) }

  #[inline]
  fn box_it(self) -> BoxWidget {
    let inner = self.inner.clone();
    let widget: Stateful<BoxWidget> = Stateful {
      inner: InnerStateful {
        wid: inner.wid,
        tree_ptr: inner.tree_ptr,
        marker: PhantomData,
      },
    };
    // widget replaced self to live.
    std::mem::forget(self);

    BoxWidget {
      widget: Box::new(widget),
    }
  }
}

impl<W: Widget> Stateful<W> {
  pub fn stateful(widget: W, mut tree: Pin<&mut widget_tree::WidgetTree>) -> Self {
    let tree_ptr = NonNull::from(&*tree);
    let wid = unsafe { tree.as_mut().get_unchecked_mut() }.new_node(widget.box_it());

    Stateful {
      inner: InnerStateful {
        wid,
        tree_ptr,
        marker: PhantomData,
      },
    }
  }

  #[inline]
  pub fn id(&self) -> WidgetId { self.inner.wid }

  pub(crate) fn replace_base_with<C: FnOnce(BoxWidget) -> BoxWidget>(&mut self, ctor: C) {
    let temp = std::mem::MaybeUninit::uninit();
    let mut temp = unsafe { temp.assume_init() };
    let base = self
      .inner
      .wid
      .assert_get_mut(unsafe { self.inner.tree_ptr.as_mut() });
    std::mem::swap(&mut temp, base);
    let mut temp = ctor(temp);
    std::mem::swap(&mut temp, base);
    std::mem::forget(temp);
  }

  #[inline]
  pub fn get_state_ref(&self) -> StateRef<W> {
    StateRef {
      inner: self.inner.clone(),
    }
  }
}

impl<T: Widget> Deref for InnerStateful<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target {
    let w = self.wid.assert_get(unsafe { self.tree_ptr.as_ref() });

    Widget::dynamic_cast_ref::<T>(w)
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl<T: Widget> DerefMut for InnerStateful<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    let tree = unsafe { self.tree_ptr.as_mut() };
    self.wid.mark_changed(tree);
    let w = self.wid.assert_get_mut(tree);

    Widget::dynamic_cast_mut::<T>(w)
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl<T: Widget> Deref for Stateful<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { self.inner.deref() }
}

impl<T: Widget> DerefMut for Stateful<T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.inner.deref_mut() }
}

impl<T: Widget> Deref for StateRef<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { self.inner.deref() }
}

impl<T: Widget> DerefMut for StateRef<T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.inner.deref_mut() }
}

impl<T: Widget> Clone for InnerStateful<T> {
  fn clone(&self) -> Self {
    Self {
      wid: self.wid,
      tree_ptr: self.tree_ptr,
      marker: PhantomData,
    }
  }
}

impl<T: Widget> Clone for StateRef<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: Widget> Drop for Stateful<T> {
  fn drop(&mut self) {
    let tree = unsafe { self.inner.tree_ptr.as_mut() };
    let wid = self.inner.wid;
    if Some(wid) != tree.root() && wid.parent(tree).is_none() {
      log::warn!("The stateful widget not add into widget tree.");
      wid.remove(tree);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let stateful = Stateful::stateful(Text("Hello".to_string()), tree.as_mut());
    {
      stateful.get_state_ref().0 = "World!".to_string();
    }
    assert!(
      tree
        .changed_widgets()
        .get(&stateful.get_state_ref().inner.wid)
        .is_some()
    );
  }

  #[test]
  fn inherit_from_stateful() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Stateful::stateful(Text("Hello".to_string()), tree.as_mut());
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id.dynamic_cast_ref::<KeyDetect>(tree);
    assert!(key_back.is_some());
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let stateful = Stateful::stateful(Text("hello".to_string()), tree.as_mut());
    (0..128).for_each(|_| unsafe {
      tree
        .as_mut()
        .get_unchecked_mut()
        .new_node(Text("".to_string()).box_it());
    });

    assert_eq!(stateful.0, "hello")
  }
}
