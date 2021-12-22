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
//! needn't write any logic code to support stateful, and shouldn't.

//! # Example
//! This example implement a rectangle widget which support change its size and
//! fill color.
//! ```
//! # #![feature(trivial_bounds, negative_impls)]
//! # use ribir::prelude::*;
//!
//! #[stateful]
//! struct Rectangle {
//!   #[state]
//!   size: Size,
//!   #[state]
//!   color: Color,
//! }
//!
//! impl CombinationWidget for Rectangle {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     BoxDecoration {
//!       background: Some(self.color.clone().into()),
//!       ..Default::default()
//!     }
//!     .have(
//!       SizedBox::from_size(self.size).box_it()
//!     )
//!     .box_it()
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
//! specify `custom` meta to `#[stateful(custom=XXXName)]` attr. This tell
//! Ribir, "I want to implement RenderWidget/CombinationWidget for the stateful
//! widget by myself instead of direct derive from the stateless version, and
//! specify name by myself. This is useful when you implement a widget and the
//! stateless version is useless and the widget self has behavior to change its
//! state. For example the [`Checkbox`](crate::prelude::Checkbox) widget."

//! ```
//! # #![feature(trivial_bounds, negative_impls)]
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
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     let rect = self.borrow();
//!     let mut state_ref = self.ref_cell();
//!     BoxDecoration {
//!       background: Some(rect.color.clone().into()),
//!       ..Default::default()
//!     }
//!     .have(
//!       SizedBox::from_size(rect.size)
//!         .on_tap(move |_| {
//!           state_ref.borrow_mut().color = Color::BLACK;
//!         })
//!        .box_it()
//!    )
//!     .box_it()
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
pub trait Stateful {
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

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget.
///
/// Remember it assume you changed the widget back of this reference if you
/// mutably borrow this ref and try to use a mutable reference of the stateful
/// widget. No matter if you really modify it.
pub struct StateRefCell<W>(StatefulImpl<W>);

// todo: remove refCell and use Rc::get_mut
pub struct StatefulImpl<W>(Rc<RefCell<AttrWidget<W>>>);

pub(crate) trait StateTrigger {
  fn trigger_change(&self);
}

#[derive(Clone)]
pub struct StateChange<T: Clone> {
  pub before: T,
  pub after: T,
}
pub(crate) struct TreeInfo {
  // use rc pointer replace NonNull pointer
  pub tree: NonNull<widget_tree::WidgetTree>,
  pub id: WidgetId,
}

impl<W: CloneStates> Stateful for StatefulImpl<W> {
  type RawWidget = W;
  fn ref_cell(&self) -> StateRefCell<Self::RawWidget> { StateRefCell(StatefulImpl(self.0.clone())) }
}

#[derive(Default)]
pub struct StateAttr {
  pub(crate) tree_info: Option<TreeInfo>,
  subject: Option<LocalSubject<'static, (), ()>>,
}

impl<W: CloneStates> Clone for StateRefCell<W> {
  fn clone(&self) -> Self { self.0.ref_cell() }
}

impl<W: SingleChildWidget> SingleChildWidget for StatefulImpl<W> {}

impl<W: MultiChildWidget> MultiChildWidget for StatefulImpl<W> {}

impl<W> AttrsAccess for StatefulImpl<W> {
  fn get_attrs(&self) -> Option<AttrRef<Attributes>> { Some(self.attrs()) }

  fn get_attrs_mut(&mut self) -> Option<AttrRefMut<Attributes>> { Some(self.attrs_mut()) }
}

impl<W> Attrs for StatefulImpl<W> {
  fn attrs(&self) -> AttrRef<Attributes> {
    let attrs = Ref::map(self.0.borrow(), |w| &w.attrs);
    AttrRef::CellRef(attrs)
  }

  fn attrs_mut(&mut self) -> AttrRefMut<Attributes> {
    let attrs = RefMut::map(self.0.borrow_mut(), |w| &mut w.attrs);
    AttrRefMut::CellRef(attrs)
  }
}

impl<W> AttachAttr for StatefulImpl<W> {
  type W = Self;

  fn into_attr_widget(self) -> Self::W { self }
}

impl<W: CloneStates + 'static> StatefulImpl<W> {
  // Convert a widget to a stateful widget, only called by framework. Maybe you
  // want [`into_stateful`](IntoStateful::into_stateful)
  pub fn new(w: W) -> Self {
    let mut attrs: Attributes = <_>::default();
    attrs.insert(StateAttr::default());
    StatefulImpl(Rc::new(RefCell::new(AttrWidget { widget: w, attrs })))
  }

  #[inline]
  pub fn borrow(&self) -> Ref<AttrWidget<W>> { self.0.borrow() }

  #[inline]
  pub fn borrow_mut(&self) -> RefMut<AttrWidget<W>> { self.0.borrow_mut() }

  /// Event emitted when this widget modified. No mather if the widget really

  pub fn change_stream(&self) -> LocalSubject<'static, (), ()> {
    self
      .0
      .borrow_mut()
      .attrs
      .find_mut::<StateAttr>()
      .unwrap()
      .state_subject()
  }

  /// Pick a field change stream from the widget, only if this state is really
  /// changed, and it detected by [`StatePartialEq`](StatePartialEq).
  pub fn state_change<T: Clone + StatePartialEq + 'static>(
    &self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()> {
    let state_ref = self.ref_cell();
    let v = pick(&self.0.borrow().widget);
    let init = StateChange { before: v.clone(), after: v };
    self
      .change_stream()
      .scan_initial(init, move |mut init, _| {
        init.before = init.after;
        init.after = pick(&*state_ref.borrow());
        init
      })
      .filter(|change| !change.before.eq(&change.after))
  }
}

impl<W: CloneStates> StateRefCell<W> {
  pub fn borrow(&self) -> Ref<AttrWidget<W>> { self.0.0.borrow() }

  /// Mutably borrows the stateful widget behind the state ref.
  ///
  /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived
  /// from it exit scope. The stateful widget cannot be borrowed while this
  /// borrow is active.
  ///
  /// If the returned mutable reference is really mutable access, state change
  /// will be trigger and the stateful widget also mark as changed at the end of
  /// this borrow exit.
  pub fn borrow_mut(&self) -> StateRefMut<W> {
    StateRefMut {
      ref_mut: self.0.0.borrow_mut(),
      deref_mut_occur: false,
      silent: false,
      host: self.0.0.clone(),
    }
  }

  /// Mutably borrows the stateful widget behind the state ref.
  ///
  /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived
  /// from it exit scope. The stateful widget cannot be borrowed while this
  /// borrow is active.
  ///
  /// If the returned mutable reference is really mutable access, state change
  /// will be trigger at the end of this borrow exit. The only difference from
  /// `borrow_mut` is the `silent_mut` will not effect the widget.
  pub fn silent_mut(&self) -> StateRefMut<W> {
    let mut ref_mut = self.borrow_mut();
    ref_mut.silent = true;
    ref_mut
  }

  pub fn ref_cell(&self) -> Self { self.clone() }

  pub fn change_stream(&self) -> LocalSubject<'static, (), ()>
  where
    W: 'static,
  {
    self.0.change_stream()
  }
}

pub struct StateRefMut<'a, W: 'static> {
  ref_mut: RefMut<'a, AttrWidget<W>>,
  deref_mut_occur: bool,
  silent: bool,
  host: Rc<RefCell<AttrWidget<W>>>,
}

impl<'a, W> StateRefMut<'a, W> {
  pub fn silent(&mut self) -> &mut Self {
    self.silent = true;
    self
  }
}

impl<'a, W: 'static> Drop for StateRefMut<'a, W> {
  fn drop(&mut self) {
    if !self.deref_mut_occur {
      return;
    }

    let mut attrs = self.ref_mut.attrs_mut();
    let state_attr = attrs.find_mut::<StateAttr>().unwrap();
    if let Some(TreeInfo { mut tree, id }) = state_attr.tree_info {
      let tree = unsafe { tree.as_mut() };
      tree.add_state_trigger(Box::new(self.host.clone()));
      if !self.silent {
        id.mark_changed(tree);
      }
    }
  }
}

impl<'a, W> std::ops::Deref for StateRefMut<'a, W> {
  type Target = RefMut<'a, AttrWidget<W>>;
  fn deref(&self) -> &Self::Target { &self.ref_mut }
}

impl<'a, W> std::ops::DerefMut for StateRefMut<'a, W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.deref_mut_occur = true;
    &mut self.ref_mut
  }
}

impl StateAttr {
  pub fn id(&self) -> Option<WidgetId> { self.tree_info.as_ref().map(|info| info.id) }

  pub(crate) fn assign_id(&mut self, id: WidgetId, tree: NonNull<WidgetTree>) {
    debug_assert!(self.tree_info.is_none());
    self.tree_info = Some(TreeInfo { tree, id })
  }

  fn state_subject(&mut self) -> LocalSubject<'static, (), ()> {
    self.subject.get_or_insert_with(<_>::default).clone()
  }
}

impl<W: CombinationWidget> CombinationWidget for StatefulImpl<W> {
  #[inline]
  fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget { self.0.borrow().build(ctx) }
}

impl<W: RenderWidget> RenderWidget for StatefulImpl<W> {
  type RO = W::RO;

  #[inline]
  fn create_render_object(&self) -> Self::RO {
    RenderWidget::create_render_object(&*self.0.borrow())
  }
}

impl<W: CloneStates> CloneStates for StatefulImpl<W> {
  type States = W::States;
  fn clone_states(&self) -> Self::States { self.0.borrow().clone_states() }
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

impl<W> StateTrigger for Rc<RefCell<AttrWidget<W>>> {
  fn trigger_change(&self) {
    // safety: borrow the inner trigger to notify state change and never modify the
    // StatefulImpl
    let attr: Option<&mut StateAttr> = self
      .borrow_mut()
      .attrs_mut()
      .find_mut::<StateAttr>()
      .map(|attr| unsafe { std::mem::transmute(attr) });

    if let Some(StateAttr { subject: Some(sbj), .. }) = attr {
      sbj.next(())
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
    let stateful = Text { text: "Hello".to_string() }.into_stateful();
    {
      stateful.ref_cell().borrow_mut().text = "World!".to_string();
    }
    assert_eq!(stateful.borrow().text, "World!");
  }

  #[test]
  fn downcast() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Text { text: "Hello".to_string() }.into_stateful();
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id.get(tree).and_then(|w| (w as &dyn AttrsAccess).get_key());
    assert!(key_back.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let sized_box = SizedBox::from_size(Size::new(100., 100.)).into_stateful();
    sized_box
      .change_stream()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    sized_box.state_change(|w| w.size).subscribe(move |size| {
      *c_changed_size.borrow_mut() = size.after;
    });

    let state = sized_box.ref_cell();
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    tree.set_root(sized_box.box_it(), &mut render_tree);

    // Borrow mut but not use it
    {
      let _ = state.borrow_mut().size;
    }
    assert_eq!(*notified_count.borrow(), 0);
    assert_eq!(tree.changed_widgets().len(), 0);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));
    {
      state.borrow_mut().size = Size::new(1., 1.);
    }
    tree.all_state_change_notify();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(tree.changed_widgets().len(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    #[derive(Debug)]
    struct TestWidget;

    impl CombinationWidget for TestWidget {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        SizedBox::from_size(Size::new(100., 100.))
          .into_stateful()
          .box_it()
      }
    }

    let mut wnd = window::Window::without_render(TestWidget.box_it(), Size::new(500., 500.));
    wnd.render_ready();
    let tree = wnd.widget_tree();
    assert_eq!(tree.root().unwrap().descendants(&*tree).count(), 2);
  }
}
