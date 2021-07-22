//! ## Stateless and Stateful
//! As default, In Ribir, every widget is stateless, just present like what you
//! declare and no interactive. That mean when you change the data of this
//! widget, the presentation of this widget will not change.

//! But Ribir provide a common method to convert a widget from sateless to
//! stateful if a widget need repaint or relayout to respond to some widget
//! change. This depends on [`Stateful`][Stateful] and
//! [`IntoStateful`][IntoStateful]
//! Use the `#[stateful]` attr  to the widget and mark what fields is state
//! field by `#[state]`. Those will provide a stateful version widget named
//! `StatefulXXX` which just a tuple struct wrap the
//! [`StatefulImpl`][StatefulImpl] with the stateless version and implement
//! [`IntoStateful`][IntoStateful]  for the stateless version widget. We
//! needn't write some logic code to support stateful, and shouldn't.

//! # Example
//! This example implement a rectangle widget which support change its size and
//! fill color.
//! ```
//! # #![feature(trivial_bounds)]
//! # use ribir::prelude::*;
//!
//! #[stateful]
//! #[derive(Widget)]
//! struct Rectangle {
//!   #[state]
//!   size: Size,
//!   #[state]
//!   color: Color,
//! }
//!
//! impl CombinationWidget for Rectangle {
//!   fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> {
//!     SizedBox::empty_box(self.size)
//!       .with_background(self.color.clone().into())
//!      .box_it()
//!   }
//! }
//!
//! let rect = Rectangle {
//!   size: Size::new(100., 100.),
//!   color: Color::RED,
//! }
//! // Rectangle support convert to stateful now.
//! .into_stateful();
//!
//! let mut state_ref = rect.ref_cell();
//! rect.on_tap(move |_| {
//!   state_ref.borrow_mut().color = Color::BLACK;
//! });
//! ```
//! In the above example, we implement a widget `Rectangle`, and use it to
//! change its color when user tapped. How to do if we want this behavior  as a
//! part of the rectangle itself. In other word, a stateless `Rectangle` is
//! useless, we only need a stateful `Rectangle`. To implement it, we can
//! specify `custom` meta to `#[stateful(custom)]` attr. This tell Ribir, "I
//! want to implement the stateful widget by myself instead of direct derive
//! from the stateless version."

//! ```
//! # #![feature(trivial_bounds)]
//! # use ribir::prelude::*;
//!
//! #[stateful(custom)]
//! struct Rectangle {
//!   #[state]
//!   size: Size,
//!   #[state]
//!   color: Color,
//! }
//!
//! impl CombinationWidget for StatefulRectangle {
//!   fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> {
//!     let rect = self.as_ref();
//!     let mut state_ref = self.ref_cell();
//!     SizedBox::empty_box(rect.size)
//!       .with_background(rect.color.clone().into())
//!       .on_tap(move |_| {
//!         state_ref.borrow_mut().color = Color::BLACK;
//!       })
//!      .box_it()
//!   }
//! }
//!
//! // Remember call the 'into_stateful', the `Rectangle` is not a widget but
//! // `StatefulRectangle` is.
//! let rect = Rectangle {
//!   size: Size::new(100., 100.),
//!   color: Color::RED,
//! }.into_stateful();
//! ```

use crate::{prelude::*, widget::widget_tree::WidgetTree};
use rxrust::prelude::*;
use std::{
  any::Any,
  cell::{Ref, RefCell, RefMut},
  mem::ManuallyDrop,
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
  type RawWidget: CloneStates;
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget>;
}

/// Trait for state change quick detect to reduce update render object, this is
/// different than equality comparisons. And no strictly rules to follow, just
/// need to make sure it's implementation is cheap, some complexity struct can
/// always return 'false'.
pub trait StatePartialEq<Rhs: ?Sized = Self> {
  fn eq(&self, other: &Rhs) -> bool;
}

/// Clone the states from the widget.
pub trait CloneStates {
  type States: StatePartialEq;
  fn clone_states(&self) -> Self::States;
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
  fn state_info(&self) -> Option<StateInfo> { Some(self.ref_cell().info) }
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

impl<W: CloneStates> Stateful for StatefulImpl<W>
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

impl<W: CloneStates> IntoStateful for StatefulImpl<W>
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

impl<W: CloneStates + 'static> StatefulImpl<W> {
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
      info: self.info.clone(),
      ref_mut: ManuallyDrop::new(self.inner_widget.0.borrow_mut()),
    }
  }
}

pub struct StateRefMut<'a, W: 'static> {
  ref_mut: ManuallyDrop<RefMut<'a, W>>,
  info: StateInfo,
}

impl<'a, W> Drop for StateRefMut<'a, W> {
  fn drop(&mut self) {
    let Self { ref_mut, info } = self;
    // Safety drop the RefMut first , will never borrow it.
    unsafe { ManuallyDrop::drop(ref_mut) };

    if info.0.borrow().subject.is_some() {
      info.state_subject().next(())
    }

    let borrowed = info.0.borrow_mut();
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
  fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> { self.0.borrow().build(ctx) }
}

impl<W: CloneStates> CloneStates for RcWidget<W> {
  type States = W::States;
  fn clone_states(&self) -> Self::States { self.0.borrow().clone_states() }
}

impl<W: RenderWidget> RenderWidget for RcWidget<W> {
  type RO = W::RO;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.0.borrow().create_render_object() }
}

macro state_partial_impl($($ty: ty)*) {
  $(impl StatePartialEq for $ty {
    #[inline]
    fn eq(&self, other: &Self) -> bool { self == other }
  })*
}

macro state_partial_for_collection($($ty:ident <$($g:ident),*>),*) {
  $(impl<$($g),*> StatePartialEq for $ty<$($g),*> {
    #[inline]
    fn eq(&self, _: &Self) -> bool { false }
  })*
}

state_partial_impl! {
  () usize u8 u16 u32 u64 u128
  isize i8 i16 i32 i64 i128
  f32 f64 String bool
}

use std::collections::{
  btree_map::BTreeMap, btree_set::BTreeSet, hash_map::HashMap, linked_list::LinkedList,
};
state_partial_for_collection!(Vec<T>, LinkedList<T>, HashMap<K, V>, BTreeMap<K, V>, BTreeSet<K>);

impl<T: StatePartialEq> StatePartialEq<Self> for Option<T> {
  fn eq(&self, other: &Self) -> bool {
    match self {
      Some(lhs) => match other {
        Some(rhs) => lhs.eq(rhs),
        None => false,
      },
      None => other.is_none(),
    }
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

    let key_back = id.get(tree).and_then(|w| w.find_attr::<Key>());
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
      fn build(&self, _: &mut BuildCtx) -> Box<dyn Widget> {
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
