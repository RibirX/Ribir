use crate::prelude::*;
use std::{marker::PhantomData, pin::Pin, ptr::NonNull};

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
#[derive(Clone, Copy)]
pub struct StateRef<T> {
  wid: WidgetId,
  tree: NonNull<widget_tree::WidgetTree>,
  _type: PhantomData<*const T>,
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
pub struct StatefulWidget {
  wid: WidgetId,
  tree: NonNull<widget_tree::WidgetTree>,
}

impl InheritWidget for StatefulWidget {
  #[inline]
  fn base_widget(&self) -> &dyn Widget { self.wid.assert_get(unsafe { self.tree.as_ref() }) }

  #[inline]
  fn base_widget_mut(&mut self) -> &mut dyn Widget {
    self.wid.assert_get_mut(unsafe { self.tree.as_mut() })
  }
}

impl_widget_for_inherit_widget!(StatefulWidget);

impl StatefulWidget {
  pub fn stateful<W: Widget>(
    widget: W,
    mut tree: Pin<&mut widget_tree::WidgetTree>,
  ) -> (BoxWidget, StateRef<W>) {
    let tree_ptr = NonNull::from(&*tree);
    let widget = inherit(
      widget.box_it(),
      |widget| {
        let wid = unsafe { tree.as_mut().get_unchecked_mut() }.new_node(widget);
        StatefulWidget {
          wid,
          tree: tree_ptr,
        }
      },
      |_| {},
    );

    let stateful = Widget::dynamic_cast_ref::<StatefulWidget>(&widget).unwrap();
    let ptr = StateRef {
      wid: stateful.wid,
      tree: tree_ptr,
      _type: PhantomData,
    };
    (widget, ptr)
  }

  #[inline]
  pub fn id(&self) -> WidgetId { self.wid }

  pub(crate) fn replace_base_with<C: FnOnce(BoxWidget) -> BoxWidget>(&mut self, ctor: C) {
    let temp = std::mem::MaybeUninit::uninit();
    let mut temp = unsafe { temp.assume_init() };
    let base = self.wid.assert_get_mut(unsafe { self.tree.as_mut() });
    std::mem::swap(&mut temp, base);
    let mut temp = ctor(temp);
    std::mem::swap(&mut temp, base);
    std::mem::forget(temp);
  }
}

impl<T: 'static> std::ops::Deref for StateRef<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target {
    let w = self.wid.assert_get(unsafe { self.tree.as_ref() });

    Widget::dynamic_cast_ref::<T>(w)
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl<T: 'static> std::ops::DerefMut for StateRef<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    let tree = unsafe { self.tree.as_mut() };
    self.wid.mark_changed(tree);
    let w = self.wid.assert_get_mut(tree);

    Widget::dynamic_cast_mut::<T>(w)
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl Drop for StatefulWidget {
  fn drop(&mut self) {
    let tree = unsafe { self.tree.as_mut() };
    if Some(self.wid) != tree.root() && self.wid.parent(tree).is_none() {
      log::warn!("The stateful widget not add into widget tree.");
      self.wid.remove(tree);
    }
  }
}

impl std::borrow::Borrow<dyn Widget> for NonNull<BoxWidget> {
  #[inline]
  fn borrow(&self) -> &dyn Widget { unsafe { self.as_ref() } }
}

impl std::borrow::BorrowMut<dyn Widget> for NonNull<BoxWidget> {
  #[inline]
  fn borrow_mut(&mut self) -> &mut dyn Widget { unsafe { self.as_mut() } }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let (_, mut cell_ref) = StatefulWidget::stateful(Text("Hello".to_string()), tree.as_mut());
    {
      cell_ref.0 = "World!".to_string();
    }
    assert!(tree.changed_widgets.get(&cell_ref.wid).is_some());
  }

  #[test]
  fn inherit_from_stateful() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let (stateful, _) = StatefulWidget::stateful(Text("Hello".to_string()), tree.as_mut());
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id
      .get_mut(tree)
      .and_then(|w| Widget::dynamic_cast_ref::<KeyDetect>(w));
    assert!(key_back.is_some());
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let (_, ptr_ref_) = StatefulWidget::stateful(Text("hello".to_string()), tree.as_mut());
    (0..128).for_each(|_| unsafe {
      tree
        .as_mut()
        .get_unchecked_mut()
        .new_node(Text("".to_string()).box_it());
    });

    assert_eq!(ptr_ref_.0, "hello")
  }
}
