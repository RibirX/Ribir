use crate::{prelude::*, widget::widget_tree::WidgetTree};
use rxrust::prelude::*;
use std::{
  any::Any,
  cell::{Ref, RefCell, RefMut},
  ptr::NonNull,
  rc::Rc,
};

/// Widget witch can be referenced and modified across `StateRefCell`
///
/// # Notice
///
/// `Stateful` trait can only implement for raw widget which not attach any
/// attributes on.
pub trait Stateful: Widget {
  type RawWidget;
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget>;
}

/// Convert a stateless widget to stateful which can provide a `StateRefCell`
/// to use to modify the states of the widget.
pub trait IntoStateful {
  type S: Stateful;
  fn into_stateful(self) -> Self::S;
}

/// Detect if a widget is stateful. If a widget is stateful return the state
/// info else return none-value.
pub trait StateDetect {
  fn state_info(&self) -> Option<StateInfo>;
}

impl<W: Widget> StateDetect for W {
  default fn state_info(&self) -> Option<StateInfo> { None }
}

impl<W: Stateful> StateDetect for W {
  fn state_info(&self) -> Option<StateInfo> { Some(self.ref_cell().info.clone()) }
}

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back of this reference if you
/// mutably borrow this pointer. No matter if you really modify it.
pub struct StateRefCell<W> {
  info: StateInfo,
  inner_widget: RcWidget<W>,
}

#[derive(Widget, RenderWidget, CombinationWidget)]
pub struct StatefulImpl<W> {
  #[proxy]
  widget: RcWidget<W>,
  info: StateInfo,
}

#[derive(Widget)]
pub struct RcWidget<W>(Rc<RefCell<W>>);

#[derive(Clone, Default)]
pub struct StateInfo(Rc<RefCell<InnerInfo>>);

#[derive(Clone)]
pub struct StateChange<T: Clone> {
  pub before: T,
  pub after: T,
}
pub(crate) struct TreeInfo {
  pub tree: NonNull<widget_tree::WidgetTree>,
  pub id: WidgetId,
}

impl<W: 'static> Widget for StatefulImpl<W> {
  default fn attrs_ref(&self) -> Option<AttrsRef> { None }

  default fn attrs_mut(&mut self) -> Option<AttrsMut> { None }
}

impl<W> Stateful for StatefulImpl<W>
where
  Self: Widget,
{
  type RawWidget = W;
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget> {
    StateRefCell {
      info: self.info.clone(),
      inner_widget: self.widget.clone(),
    }
  }
}

impl<W> IntoStateful for StatefulImpl<W>
where
  Self: Widget,
{
  type S = Self;
  #[inline]
  fn into_stateful(self) -> Self::S { self }
}

#[derive(Default)]
struct InnerInfo {
  pub(crate) tree_info: Option<TreeInfo>,
  subject: Option<LocalSubject<'static, (), ()>>,
}

impl<W> Clone for RcWidget<W> {
  #[inline]
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<W: 'static> Clone for StateRefCell<W> {
  fn clone(&self) -> Self {
    Self {
      info: self.info.clone(),
      inner_widget: self.inner_widget.clone(),
    }
  }
}

impl<W: 'static> StatefulImpl<W> {
  pub fn new(w: W) -> Self {
    Self {
      info: <_>::default(),
      widget: RcWidget(Rc::new(RefCell::new(w))),
    }
  }

  #[inline]
  pub fn as_ref(&self) -> Ref<W> { self.widget.0.borrow() }

  #[inline]
  pub fn as_mut(&mut self) -> RefMut<W> { self.widget.0.borrow_mut() }

  /// Event emitted when this widget modified.
  pub fn change_stream(
    &mut self,
  ) -> impl LocalObservable<'static, Item = StateRefCell<W>, Err = ()> {
    let ref_cell = self.ref_cell();
    self.info.state_subject().map(move |_| ref_cell.clone())
  }

  /// Pick a field change stream from the host widget.
  pub fn state_change<T: Clone + 'static>(
    &mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()> {
    let v = pick(&*self.widget.0.borrow());
    let init = StateChange { before: v.clone(), after: v };
    self
      .change_stream()
      .scan_initial(init, move |mut init, value| {
        init.before = init.after;
        init.after = pick(&*value.borrow());
        init
      })
  }
}

impl<W: 'static> StateRefCell<W> {
  pub fn borrow(&self) -> Ref<W> { self.inner_widget.0.borrow() }

  pub fn borrow_mut(&mut self) -> StateRefMut<W> {
    StateRefMut {
      attr: self.info.clone(),
      ref_mut: self.inner_widget.0.borrow_mut(),
    }
  }
}

pub struct StateRefMut<'a, W: 'static> {
  ref_mut: RefMut<'a, W>,
  attr: StateInfo,
}

impl<'a, W> Drop for StateRefMut<'a, W> {
  fn drop(&mut self) {
    let Self { ref_mut, attr } = self;
    // Safety drop the RefMut first , will never borrow it.
    std::mem::drop(ref_mut);

    if attr.0.borrow().subject.is_some() {
      attr.state_subject().next(())
    }

    let borrowed = attr.0.borrow_mut();
    if let Some(TreeInfo { mut tree, id }) = borrowed.tree_info {
      id.mark_changed(unsafe { tree.as_mut() });
    }
  }
}

impl<'a, W> std::ops::Deref for StateRefMut<'a, W> {
  type Target = RefMut<'a, W>;
  fn deref(&self) -> &Self::Target { &self.ref_mut }
}

impl<'a, W> std::ops::DerefMut for StateRefMut<'a, W> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ref_mut }
}

impl StateInfo {
  pub fn id(&self) -> Option<WidgetId> { self.0.borrow().tree_info.as_ref().map(|info| info.id) }

  pub(crate) fn assign_id(&self, id: WidgetId, tree: NonNull<WidgetTree>) {
    let mut info = self.0.borrow_mut();
    debug_assert!(info.tree_info.is_none());
    info.tree_info = Some(TreeInfo { id, tree })
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let stateful = Text("Hello".to_string()).into_stateful();
    {
      stateful.ref_cell().borrow_mut().0 = "World!".to_string();
    }
    assert_eq!(stateful.as_ref().0, "World!");
  }

  #[test]
  fn downcast() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Text("Hello".to_string()).into_stateful();
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id.get(tree).and_then(|w| w.widget.find_attr::<Key>());
    assert!(key_back.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let mut sized_box = SizedBox::empty_box(Size::new(100., 100.)).into_stateful();
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
      fn build(&self, _: &mut BuildCtx) -> BoxWidget {
        SizedBox::empty_box(Size::new(100., 100.))
          .into_stateful()
          .box_it()
      }
    }

    let mut wnd = window::Window::without_render(TestWidget, Size::new(500., 500.));
    wnd.render_ready();
    let tree = wnd.widget_tree();
    assert_eq!(tree.count(), 2);
  }
}
