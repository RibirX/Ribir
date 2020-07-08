use crate::{prelude::*, widget::inherit_widget};
use std::{marker::PhantomData, pin::Pin, ptr::NonNull};

/// A pointer of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back this pointer if you mutably
/// dereference this pointer. No matter if you really modify it.
///
/// ## Safety
/// Because `StatefulPtr` can only be constructed in `Combination::build`
/// method, the only way to live longer than `build` method scope it capture by
/// some closure of widget that construct in the same scope, so the pointer will
/// have same lifetime as the widget capture it. And framework guarantee the
/// widgets constructed in the same `build` method  have same lifetime.
///
/// Maybe panic if widget impl the `Drop` trait, and call some closure in its
/// `drop` method,  the captured `StatefulPtr` maybe is dangling.
#[derive(Clone, Copy)]
pub struct StatefulPtr<T> {
  wid: WidgetId,
  tree: NonNull<widget_tree::WidgetTree>,
  widget: NonNull<dyn Widget>,
  _type: PhantomData<*const T>,
}

/// `Stateful` erased widget type info and used only as common identify type for
/// all stateful widget.
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
  widget: NonNull<dyn Widget>,
}

pub fn widget_into_stateful<W: Widget>(
  widget: W,
  mut tree: Pin<&mut widget_tree::WidgetTree>,
) -> (StatefulWidget, StatefulPtr<W>) {
  let box_widget = widget.box_it();
  let widget = NonNull::from(&*box_widget.widget);
  let wid = unsafe { tree.as_mut().get_unchecked_mut() }.new_node(box_widget);
  (
    StatefulWidget { wid, widget },
    StatefulPtr {
      wid,
      tree: NonNull::from(tree.into_ref().get_ref()),
      widget,
      _type: PhantomData,
    },
  )
}

inherit_widget!(StatefulWidget, widget);

impl StatefulWidget {
  #[inline]
  pub fn id(&self) -> WidgetId { self.wid }
}

impl<T: 'static> std::ops::Deref for StatefulPtr<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target {
    unsafe { self.widget.as_ref() as &dyn Widget }
      .dynamic_ref::<T>()
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl<T: 'static> std::ops::DerefMut for StatefulPtr<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.wid.mark_changed(unsafe { self.tree.as_mut() });
    unsafe { self.widget.as_mut() as &mut dyn Widget }
      .dynamic_mut::<T>()
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl std::borrow::Borrow<dyn Widget> for NonNull<dyn Widget> {
  #[inline]
  fn borrow(&self) -> &dyn Widget { unsafe { self.as_ref() } }
}

impl std::borrow::BorrowMut<dyn Widget> for NonNull<dyn Widget> {
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
    let (_, mut cell_ref) = widget_into_stateful(Text("Hello".to_string()), tree.as_mut());
    {
      cell_ref.0 = "World!".to_string();
    }
    assert!(tree.changed_widgets.get(&cell_ref.wid).is_some());
  }

  #[test]
  fn inherit_from_stateful() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let (stateful, _) = widget_into_stateful(Text("Hello".to_string()), tree.as_mut());
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id.get_mut(tree).and_then(|w| w.dynamic_ref::<KeyDetect>());
    assert!(key_back.is_some());
  }
}
