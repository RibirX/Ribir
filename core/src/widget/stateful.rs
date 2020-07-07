use crate::{prelude::*, widget::inherit_widget};
use std::{marker::PhantomData, ptr::NonNull};

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
  widget: NonNull<BoxWidget>,
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
  widget: NonNull<BoxWidget>,
}

pub fn widget_into_stateful<W: Widget>(
  widget: W,
  ctx: &mut BuildCtx,
) -> (StatefulWidget, StatefulPtr<W>) {
  let box_widget = widget.box_it();
  let widget = NonNull::from(&box_widget);
  let wid = unsafe { ctx.tree.as_mut().new_node(box_widget) };
  (
    StatefulWidget { wid, widget },
    StatefulPtr {
      wid,
      tree: ctx.tree,
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
      .downcast_ref::<T>()
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
  }
}

impl<T: 'static> std::ops::DerefMut for StatefulPtr<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.wid.mark_changed(unsafe { self.tree.as_mut() });
    unsafe { self.widget.as_mut() as &mut dyn Widget }
      .downcast_mut::<T>()
      .unwrap_or_else(|| unreachable!("Ref type error. should never happen!"))
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
    let tree = widget_tree::WidgetTree::default();
    let mut ctx = BuildCtx {
      tree: NonNull::from(&tree),
    };
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let mut cell_ref = {
      let t = Text("Hello".to_string());
      let (_, cell_ref) = t.into_stateful(&mut ctx);
      cell_ref
    };
    {
      cell_ref.0 = "World!".to_string();
    }
    assert!(tree.changed_widgets.get(&cell_ref.wid).is_some());
  }
}
