use crate::{prelude::*, widget::attr};
use rxrust::prelude::*;
use std::{
  any::Any,
  cell::{Ref, RefCell, RefMut},
  mem::ManuallyDrop,
  ptr::NonNull,
  rc::Rc,
};

use super::widget_tree::WidgetTree;

/// This widget convert a stateless widget to stateful.
#[derive(RenderWidget, CombinationWidget)]
pub struct Stateful<W: Widget> {
  #[proxy]
  widget: RcWidget<W>,
  stateful: StatefulAttr,
  others: Option<Attrs>,
}

/// Convert a stateless widget to stateful which can provide a `StateRefCell`
/// which can be use to modify the states of the widget.
pub trait IntoStateful {
  type W: Widget;

  fn into_stateful(self, ctx: &mut BuildCtx) -> Stateful<Self::W>;
}

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back of this reference if you
/// mutably borrow this pointer. No matter if you really modify it.
///
/// ## Panics
///
/// `StateRefCell` should not live longer than its widget. Framework guarantee
/// the widgets constructed in the same `build` method  have same lifetime,  and
/// parent live longer than parent. So not pass a `StateRefCell` to its
/// ancestors, that maybe panic.
pub struct StateRefCell<W: Widget> {
  attr: StatefulAttr,
  inner_widget: RcWidget<W>,
}

#[derive(Widget)]
pub struct RcWidget<W: 'static>(Rc<RefCell<W>>);

#[derive(Clone, Default)]
pub struct StatefulAttr(pub(crate) Rc<RefCell<InnerAttr>>);

#[derive(Clone)]
pub struct StateChange<T: Clone> {
  pub before: T,
  pub after: T,
}
pub(crate) struct TreeInfo {
  pub tree: NonNull<widget_tree::WidgetTree>,
  pub id: WidgetId,
}

#[derive(Default)]
pub(crate) struct InnerAttr {
  pub(crate) tree_info: Option<TreeInfo>,
  subject: Option<LocalSubject<'static, (), ()>>,
}

impl<W: Widget> Clone for RcWidget<W> {
  #[inline]
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<W: Widget> Clone for StateRefCell<W> {
  fn clone(&self) -> Self {
    Self {
      attr: self.attr.clone(),
      inner_widget: self.inner_widget.clone(),
    }
  }
}

impl<W: Widget> Widget for Stateful<W> {
  fn attrs_ref(&self) -> Option<AttrsRef> {
    Some(AttrsRef::new(&self.stateful, self.others.as_ref()))
  }

  fn attrs_mut(&mut self) -> Option<AttrsMut> {
    Some(AttrsMut::new(&mut self.stateful, self.others.as_mut()))
  }
}

impl<W: Widget> AttachAttr for Stateful<W> {
  type W = RcWidget<W>;

  fn split_attrs(self) -> (Self::W, Option<Attrs>) {
    let mut attrs = self.others.unwrap_or(<_>::default());
    attrs.front_push_attr(self.stateful);
    (self.widget, Some(attrs))
  }
}

impl<W: Widget> Stateful<W> {
  // pub fn stateful<A: AttachAttr>(widget: A) -> Self {
  //   let AttrWidget {
  //     widget,
  //     major,
  //     others,
  //   } = widget.into_attr_widget::<StatefulAttr>();

  //   if let Some(major) = major {
  //     if let Some(w) = Any::downcast_ref::<RcWidget<W>>(&widget) {
  //       let rcw = std::mem::MaybeUninit::uninit();
  //       let ptr: *mut RcWidget<W> = rcw.as_mut_ptr();
  //       unsafe { ptr.copy_from(w as *const RcWidget<W>, 1) }
  //       std::mem::forget(widget);
  //       let widget = unsafe { rcw.assume_init() };
  //       Stateful {
  //         widget,
  //         stateful: major,
  //         others,
  //       }
  //     } else {
  //       unimplemented!()
  //     }
  //   } else {
  //     unimplemented!()
  //   }
  // }

  #[inline]
  pub fn ref_cell(&self) -> StateRefCell<W> {
    StateRefCell {
      attr: self.stateful.clone(),
      inner_widget: self.widget.clone(),
    }
  }

  /// Event emitted when this widget modified.
  pub fn change_stream(
    &mut self,
  ) -> impl LocalObservable<'static, Item = StateRefCell<W>, Err = ()> {
    let ref_cell = self.ref_cell();
    self.stateful.state_subject().map(move |_| ref_cell.clone())
  }

  /// Pick a field change stream from the host widget.
  pub fn state_change<T: Clone + 'static>(
    &mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()> {
    let v = pick(&*self.widget.0.borrow());
    let init = StateChange {
      before: v.clone(),
      after: v,
    };
    self
      .change_stream()
      .scan_initial(init, move |mut init, value| {
        init.before = init.after;
        init.after = pick(&*value.borrow());
        init
      })
  }
}

impl<W: Widget> StateRefCell<W> {
  pub(crate) fn new(attr: StatefulAttr, widget: RcWidget<W>) -> Self {
    Self {
      attr,
      inner_widget: widget,
    }
  }

  pub fn borrow(&self) -> Ref<W> { self.inner_widget.0.borrow() }

  pub fn borrow_mut(&mut self) -> StateRefMut<W> {
    StateRefMut {
      attr: self.attr.clone(),
      ref_mut: ManuallyDrop::new(self.inner_widget.0.borrow_mut()),
    }
  }
}

pub struct StateRefMut<'a, W: Widget> {
  ref_mut: ManuallyDrop<RefMut<'a, W>>,
  attr: StatefulAttr,
}

impl<'a, W: Widget> Drop for StateRefMut<'a, W> {
  fn drop(&mut self) {
    // Safety drop the RefMut first , will never borrow it.
    unsafe { ManuallyDrop::drop(&mut self.ref_mut) };

    if self.attr.0.borrow().subject.is_some() {
      self.attr.state_subject().next(())
    }

    let mut borrowed = self.attr.0.borrow_mut();
    if let Some(TreeInfo { mut tree, id }) = borrowed.tree_info {
      id.mark_changed(unsafe { tree.as_mut() });
    }
  }
}

impl<'a, W: Widget> std::ops::Deref for StateRefMut<'a, W> {
  type Target = RefMut<'a, W>;
  fn deref(&self) -> &Self::Target { &self.ref_mut }
}

impl<'a, W: Widget> std::ops::DerefMut for StateRefMut<'a, W> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ref_mut }
}

impl StatefulAttr {
  pub fn id(&self) -> Option<WidgetId> { self.0.borrow().tree_info.as_ref().map(|info| info.id) }

  pub(crate) fn new(id: WidgetId, tree: NonNull<WidgetTree>) -> Self {
    Self(Rc::new(RefCell::new(InnerAttr {
      tree_info: Some(TreeInfo { id, tree }),
      subject: None,
    })))
  }

  fn state_subject(&mut self) -> LocalSubject<'static, (), ()> {
    self
      .0
      .borrow_mut()
      .subject
      .get_or_insert_with(<_>::default)
      .clone()
  }
}

impl<W: CombinationWidget> CombinationWidget for RcWidget<W> {
  #[inline]
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget { self.0.borrow().build(ctx) }
}
pub struct StateInnerRender<R>(R);

impl<W: RenderWidget> RenderWidget for RcWidget<W> {
  type RO = StateInnerRender<W::RO>;

  #[inline]
  fn create_render_object(&self) -> Self::RO {
    StateInnerRender(self.0.borrow().create_render_object())
  }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    (self.0.borrow_mut()).take_children()
  }
}

impl<R: RenderObject> RenderObject for StateInnerRender<R> {
  type Owner = RcWidget<R::Owner>;

  #[inline]
  fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx) {
    self.0.update(&*owner_widget.0.borrow(), ctx)
  }
  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    self.0.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.0.only_sized_by_parent() }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) { self.0.paint(ctx) }

  #[inline]
  fn transform(&self) -> Option<Transform> { self.0.transform() }
}

impl<W: AttachAttr> IntoStateful for W {
  type W = W::W;
  fn into_stateful(self, ctx: &mut BuildCtx) -> Stateful<Self::W> {
    let AttrWidget {
      widget,
      major,
      others,
    } = self.into_attr_widget::<StatefulAttr>();

    let widget = major
      .and_then(|_| Any::downcast_ref::<RcWidget<Self::W>>(&widget))
      .map(|w| {
        let rcw = std::mem::MaybeUninit::uninit();
        let ptr: *mut RcWidget<Self::W> = rcw.as_mut_ptr();
        unsafe { ptr.copy_from(w as *const RcWidget<Self::W>, 1) }
        std::mem::forget(widget);
        unsafe { rcw.assume_init() }
      })
      .unwrap_or_else(|| RcWidget(Rc::new(RefCell::new(widget))));

    let stateful = major.unwrap_or_default();

    Stateful {
      widget,
      stateful,
      others,
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
    let stateful = Stateful::stateful(Text("Hello".to_string()).into_attr_widget());
    {
      stateful.ref_cell().borrow_mut().0 = "World!".to_string();
    }
    assert_eq!(&stateful.0, "World!");
  }

  #[test]
  fn downcast() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Stateful::stateful(Text("Hello".to_string()).into_attr_widget());
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id
      .get(tree)
      .and_then(|w| w.find_attr::<Key>())
      .map(|k| k.key());
    assert!(key_back.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let mut sized_box =
      Stateful::stateful(SizedBox::empty_box(Size::new(100., 100.)).into_attr_widget());

    sized_box
      .change_stream()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    sized_box.state_change(|w| w.size).subscribe(move |size| {
      *c_changed_size.borrow_mut() = size.after;
    });

    let mut state = sized_box.ref_cell();
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    tree.set_root(sized_box.box_it(), &mut render_tree);

    {
      state.borrow_mut();
      state.borrow_mut();
    }

    assert_eq!(*notified_count.borrow(), 2);
    assert_eq!(tree.changed_widgets().len(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(100., 100.));
  }

  #[test]
  fn fix_pin_widget_node() {
    #[derive(Debug, Widget)]
    struct TestWidget;

    impl CombinationWidget for TestWidget {
      fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
        SizedBox::empty_box(Size::new(100., 100.))
          .into_stateful(ctx)
          .box_it()
      }
    }

    let mut wnd = window::Window::without_render(TestWidget, Size::new(500., 500.));
    wnd.render_ready();
    let tree = wnd.widget_tree();
    assert_eq!(tree.count(), 2);
  }
}
