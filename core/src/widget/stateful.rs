use crate::prelude::*;
use std::{
  cell::{Ref, RefCell, RefMut},
  marker::PhantomData,
  pin::Pin,
  ptr::NonNull,
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
  attr: StatefulAttr,
  type_info: PhantomData<*const T>,
}

/// This widget convert a stateless widget to stateful.
pub type Stateful<T> = WidgetAttr<T, StatefulAttr>;

#[derive(Debug)]
pub struct StatefulAttr {
  tree: NonNull<widget_tree::WidgetTree>,
  pointer: Rc<RefCell<(*const dyn Widget, *mut dyn Widget)>>,
  id: WidgetId,
}

impl<W: Widget> Stateful<W> {
  #[inline]
  pub fn get_state_ref(&self) -> StateRef<W> {
    StateRef {
      attr: StatefulAttr {
        tree: self.attr.tree,
        pointer: self.attr.pointer.clone(),
        id: self.attr.id,
      },
      type_info: PhantomData,
    }
  }

  #[inline]
  pub fn id(&self) -> WidgetId { self.attr.id }
}

impl<W: Widget> StateRef<W> {
  /// ## Safety
  /// Should ensure the pointer in attr has the same type with `W`, otherwise
  /// panic occur.
  pub unsafe fn new(attr: StatefulAttr) -> Self {
    StateRef {
      attr: StatefulAttr {
        tree: attr.tree,
        pointer: attr.pointer.clone(),
        id: attr.id,
      },
      type_info: std::marker::PhantomData,
    }
  }

  pub fn borrow(&self) -> Ref<W> {
    Ref::map(self.attr.pointer.borrow(), |(p, _)| unsafe {
      &*(*p as *const W)
    })
  }

  pub fn borrow_mut(&mut self) -> RefMut<W> {
    self
      .attr
      .id
      .mark_changed(unsafe { self.attr.tree.as_mut() });
    RefMut::map(self.attr.pointer.borrow_mut(), |(_, p)| unsafe {
      &mut *(*p as *mut W)
    })
  }
}

impl<W: Widget> Stateful<W> {
  pub fn stateful<A: AttributeAttach<HostWidget = W>>(
    widget: A,
    mut tree: Pin<&mut widget_tree::WidgetTree>,
  ) -> Self {
    widget.unwrap_attr_or_else_with(|mut widget| {
      let id =
        unsafe { tree.as_mut().get_unchecked_mut() }.alloc_node(widget::PhantomWidget.box_it());
      let pointer = StatefulAttr::rc_pointer(&mut widget);
      let attr = StatefulAttr {
        id,
        pointer,
        tree: NonNull::from(&*tree),
      };
      (widget, attr)
    })
  }
}

impl StatefulAttr {
  pub(crate) fn from_id(id: WidgetId, mut tree: Pin<&mut widget_tree::WidgetTree>) -> Self {
    let widget = id.assert_get_mut(unsafe { tree.as_mut().get_unchecked_mut() });
    let pointer = Self::rc_pointer(widget);
    Self {
      pointer,
      id,
      tree: NonNull::from(&*tree),
    }
  }

  fn rc_pointer(widget: &mut BoxWidget) -> Rc<RefCell<(*const dyn Widget, *mut dyn Widget)>> {
    Rc::new(RefCell::new((
      &*widget.widget as *const _,
      &mut *widget.widget as *mut _,
    )))
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
      stateful.get_state_ref().borrow_mut().0 = "World!".to_string();
    }
    assert_eq!(&stateful.0, "World!");
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

    let key_back = id
      .get(tree)
      .and_then(|w| w.downcast_attr_widget::<Key>())
      .map(|k| k.key());
    assert!(key_back.is_some());
  }

  #[test]
  fn fix_pin_widget_node() {
    #[derive(Debug)]
    struct TestWidget;

    impl CombinationWidget for TestWidget {
      fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
        SizedBox::empty_box(Size::new(100., 100.))
          .into_stateful(ctx)
          .box_it()
      }
    }

    impl_widget_for_combination_widget!(TestWidget);

    let mut wnd = window::Window::without_render(TestWidget.box_it(), Size::new(500., 500.));
    wnd.render_ready();
    let tree = wnd.widget_tree();
    assert_eq!(tree.count(), 2);
  }
}
