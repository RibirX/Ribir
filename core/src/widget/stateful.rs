//! ## Stateless and Stateful
//! As default, In Ribir, every widget is stateless, just present like what you
//! declare and no interactive. That mean you can't modify the data of the
//! widget, the presentation of this widget is static.

//! But Ribir provide a stateful implementation version widget for every widget,
//! convert widget across ` [`IntoStateful`]!. So, in most cases you implement
//! your widget without stateful, and a stateful version will provide by Ribir.
//!
//! # Example
//! This example implement a rectangle widget which support change its size and
//! fill color.
//! ```
//! # use ribir::prelude::*;
//!
//! struct Rectangle {
//!   size: Size,
//!   color: Color,
//! }
//!
//! impl CombinationWidget for Rectangle {
//!   #[widget]
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     widget!{
//!       declare SizedBox {
//!         size: self.size,
//!         background: self.color.clone()
//!       }
//!     }
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
//! let mut state_ref = unsafe { rect.state_ref() };
//! rect.on_tap(move |_| { state_ref.color = Color::BLACK; });
//! ```
//! In the above example, we implement a widget `Rectangle`, and use it to
//! change its color when user tapped.
//!
//! How to do if the `on_tap` behavior should as a part of the rectangle
//! itself, not need to user to listen. In this case we should skip to implement
//! `CombinationWidget`, but directly implement `StatefulCombination`,

//! ```
//! # use ribir::prelude::*;
//!
//! struct Rectangle {
//!   size: Size,
//!   color: Color,
//! }
//!
//! impl StatefulCombination for Rectangle {
//!   #[widget]
//!   fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
//!     let mut this_ref = unsafe { this.state_ref() };
//!     widget!{
//!       declare SizedBox {
//!         size: this.size,
//!         background: this.color.clone(),
//!         on_tap: move |_| this_ref.color = Color::BLACK
//!       }
//!     }
//!   }
//! }
//!
//! // Remember call the 'into_stateful', the `Rectangle` is not a widget but
//! // its stateful version is.
//! let rect = Rectangle {
//!   size: Size::new(100., 100.),
//!   color: Color::RED,
//! }.into_stateful();
//! ```
//!
//! Notice, the first argument of `build` method is `Stateful<Self>` let you can
//! access self `sate_ref`, that the only different with `CombinationWidget`.

use crate::{prelude::*, widget::widget_tree::WidgetTree};
use rxrust::prelude::*;
use std::{cell::Cell, pin::Pin, ptr::NonNull};

use super::widget_tree::WidgetChangeFlags;

/// Convert a stateless widget to stateful which can provide a `StateRefCell`
/// to use to modify the states of the widget.
pub trait IntoStateful {
  type S;
  fn into_stateful(self) -> Self::S;
}

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget. Tracked the state change across if user mutable reference
/// the `StateRef` and trigger state change notify and require `ribir` to
/// rebuild or relayout inner widget.
pub struct StateRef<W>(NonNull<AttrWidget<W>>);

/// A reference of stateful widget, tracked the state change across if user
/// mutable reference the `SilentRef`. If mutable reference occur, state change
/// notify will trigger, but not effect the inner widget relayout or rebuild.
///
/// If you not very clear how `SilentRef` work, use [`StateRef`]! instead of.
pub struct SilentRef<W>(NonNull<AttrWidget<W>>);

/// A reference of stateful widget, tracked the relayout or rebuild if user
/// mutable reference the `ShallowRef`, but state change notify will not
/// trigger. Now used in animation's render change, which is just a
/// temporary change in render and not expect to change the data.
/// If you not very clear how `SilentRef` work, use [`StateRef`]! instead of.
pub struct ShallowRef<W>(NonNull<AttrWidget<W>>);

/// The stateful widget generic implementation.
pub struct Stateful<W>(Pin<Box<AttrWidget<W>>>);

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

#[derive(Default)]
pub(crate) struct StateAttr {
  pub(crate) tree_info: Option<TreeInfo>,
  subject: Option<LocalSubject<'static, (), ()>>,
  during_build: Cell<bool>,
}

impl<W> Clone for SilentRef<W> {
  fn clone(&self) -> Self { Self(self.0) }
}

impl<W> Clone for StateRef<W> {
  fn clone(&self) -> Self { Self(self.0) }
}

impl<W> Clone for ShallowRef<W> {
  fn clone(&self) -> Self { Self(self.0) }
}

impl<W> Copy for StateRef<W> {}
impl<W> Copy for SilentRef<W> {}
impl<W> Copy for ShallowRef<W> {}

impl<W> Stateful<W> {
  // Convert a widget to a stateful widget, only called by framework. Maybe you
  // want [`into_stateful`](IntoStateful::into_stateful)
  fn new(w: W) -> Self {
    let mut attrs: Attributes = <_>::default();
    attrs.insert(StateAttr::default());
    Stateful(Box::pin(AttrWidget { widget: w, attrs }))
  }

  /// Return a `StateRef` of the stateful widget, caller should careful not keep
  /// it live not longer than its widget.
  #[inline]
  pub unsafe fn state_ref(&self) -> StateRef<W> { StateRef(NonNull::from(&*self.0)) }

  /// Return a `SilentRef` of the stateful widget. Caller should careful not
  /// keep it live not longer than its widget.
  #[inline]
  pub unsafe fn silent_ref(&self) -> SilentRef<W> { SilentRef(NonNull::from(&*self.0)) }

  #[inline]
  pub unsafe fn shallow_ref(&self) -> ShallowRef<W> { ShallowRef(NonNull::from(&*self.0)) }

  /// Event emitted when this widget modified. No mather if the widget really
  #[inline]
  pub fn change_stream(&mut self) -> LocalSubject<'static, (), ()> {
    assert_state_attr(self).state_subject()
  }

  /// Pick field change stream from the widget change
  pub fn state_change<T: Clone + 'static>(
    &mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()>
  where
    Self: 'static,
  {
    let state_ref = unsafe { self.state_ref() };
    state_ref.state_change(pick)
  }

  pub(crate) fn mark_during_build(&self, flag: bool) {
    self
      .find_attr::<StateAttr>()
      .unwrap()
      .during_build
      .set(flag);
  }
}

impl<W: 'static> StateRef<W> {
  // convert a `StateRef` to `SilentRef`
  #[inline]
  pub fn silent(self) -> SilentRef<W> { SilentRef(self.0) }

  #[inline]
  pub fn shallow(self) -> ShallowRef<W> { ShallowRef(self.0) }

  /// Event emitted when this widget modified. No mather if the widget really
  #[inline]
  pub fn change_stream(&mut self) -> LocalSubject<'static, (), ()> {
    assert_state_attr(self).state_subject()
  }

  /// Pick field change stream from the widget change
  pub fn state_change<T: Clone + 'static>(
    mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()>
  where
    Self: 'static,
  {
    let v = pick(&self.widget);
    let init = StateChange { before: v.clone(), after: v };
    self.change_stream().scan_initial(init, move |mut init, _| {
      init.before = init.after;
      init.after = pick(&self);
      init
    })
  }
}

impl<W> SilentRef<W> {
  #[inline]
  pub fn change_stream(&mut self) -> LocalSubject<'static, (), ()> {
    assert_state_attr(self).state_subject()
  }

  /// Pick field change stream from the widget change
  pub fn state_change<T: Clone + 'static>(
    &mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()>
  where
    Self: 'static,
  {
    StateRef(self.0).state_change(pick)
  }
}

impl<W> std::ops::Deref for Stateful<W> {
  type Target = AttrWidget<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<W> std::ops::DerefMut for Stateful<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // Safety
    // - `Stateful` is not support clone, so as a widget it's unique and safe to get
    //   inner mutable reference。
    // - StateRef may hold a pointer of this in the `widget!` macro, ribir guarantee
    //   the generate code have not data race.
    unsafe { self.0.as_mut().get_unchecked_mut() }
  }
}

impl<W> std::ops::Deref for SilentRef<W> {
  type Target = AttrWidget<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { unsafe { self.0.as_ref() } }
}

impl<W> std::ops::DerefMut for SilentRef<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {
      assert_state_attr(self.0.as_mut()).record_change(WidgetChangeFlags::DIFFUSE);
      self.0.as_mut()
    }
  }
}

impl<W> std::ops::Deref for ShallowRef<W> {
  type Target = AttrWidget<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { unsafe { self.0.as_ref() } }
}

impl<W> std::ops::DerefMut for ShallowRef<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {
      assert_state_attr(self.0.as_mut()).record_change(WidgetChangeFlags::UNSILENT);
      self.0.as_mut()
    }
  }
}

impl<W> std::ops::Deref for StateRef<W> {
  type Target = AttrWidget<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { unsafe { self.0.as_ref() } }
}

impl<W> std::ops::DerefMut for StateRef<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe {
      assert_state_attr(self.0.as_mut()).record_change(WidgetChangeFlags::ALL);
      self.0.as_mut()
    }
  }
}

impl StateAttr {
  pub(crate) fn assign_id(&mut self, id: WidgetId, tree: NonNull<WidgetTree>) {
    debug_assert!(self.tree_info.is_none());
    self.tree_info = Some(TreeInfo { tree, id })
  }

  fn record_change(&mut self, flag: WidgetChangeFlags) {
    if let Some(TreeInfo { mut tree, id }) = self.tree_info {
      if self.during_build.get() {
        log::warn!("Modify widget state during it build child is not allowed!");
      } else {
        unsafe { tree.as_mut() }.record_change(id, flag);
      }
    }
  }

  pub(crate) fn changed_notify(&mut self) {
    if let Some(s) = self.subject.as_mut() {
      s.next(())
    }
  }

  fn state_subject(&mut self) -> LocalSubject<'static, (), ()> {
    self.subject.get_or_insert_with(<_>::default).clone()
  }
}

/// A wrap for `Stateful` to help we can implement stateful version widget for
/// for all widget, and avoid trait implement conflict.
pub(crate) struct StatefulWrap<W>(Stateful<W>);

impl<W: Render> IntoRender for Stateful<W> {
  type R = StatefulWrap<W>;

  #[inline]
  fn into_render(self) -> Self::R { StatefulWrap(self) }
}

impl<W: Compose> IntoCombination for Stateful<W> {
  type C = StatefulWrap<W>;
  #[inline]
  fn into_combination(self) -> Self::C { StatefulWrap(self) }
}

impl<W> SingleChildWidget for Stateful<W> where W: SingleChildWidget {}

impl<W> MultiChildWidget for Stateful<W> where W: MultiChildWidget {}

impl<C: Compose> Compose for StatefulWrap<C> {
  type W = C::W;
  #[inline]
  fn compose(self, ctx: &mut BuildCtx) -> Self::W { self.0.compose(ctx) }
}

impl<W: Render> Render for StatefulWrap<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.0.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.0.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.0.paint(ctx) }
}

// Implement IntoStateful for all widget

impl<W: Widget + 'static> IntoStateful for W {
  type S = Stateful<W>;
  #[inline]
  fn into_stateful(self) -> Self::S { Stateful::new(self) }
}

impl<W: IntoStateful> IntoStateful for AttrWidget<W> {
  type S = Stateful<W>;

  fn into_stateful(mut self) -> Self::S {
    self.attrs.insert(StateAttr::default());
    Stateful(Box::pin(self))
  }
}

fn assert_state_attr<W>(w: &mut AttrWidget<W>) -> &mut StateAttr {
  w.find_attr_mut::<StateAttr>()
    .expect("stateful widget must have `StateAttr`")
}

impl<W> AsAttrs for Stateful<W> {
  #[inline]
  fn as_attrs(&self) -> Option<&Attributes> { self.0.as_attrs() }

  #[inline]
  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> {
    let inner = unsafe { self.0.as_mut().get_unchecked_mut() };
    inner.as_attrs_mut()
  }
}

impl<W> AsAttrs for StatefulWrap<W>
where
  Self: Widget,
{
  #[inline]
  fn as_attrs(&self) -> Option<&Attributes> { self.0.as_attrs() }

  #[inline]
  fn as_attrs_mut(&mut self) -> Option<&mut Attributes> { self.0.as_attrs_mut() }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    // Simulate `Text` widget need modify its text in event callback. So return a
    // cell ref of the `Text` but not own it. Can use the `cell_ref` in closure.
    let stateful = Text {
      text: "Hello".into(),
      style: TextStyle::default(),
    }
    .into_stateful();
    {
      unsafe { stateful.state_ref() }.text = "World!".into();
    }
    assert_eq!(&*stateful.text, "World!");
  }

  #[test]
  fn stateful_id_check() {
    let stateful = Text {
      text: "Hello".into(),
      style: TextStyle::default(),
    }
    .into_stateful();
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let ctx = Context::new(key.box_it(), 1., None);
    let tree = &ctx.widget_tree;
    let key = tree.root().assert_get(tree).get_key();
    assert!(key.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let mut sized_box = SizedBox { size: Size::new(100., 100.) }.into_stateful();
    sized_box
      .change_stream()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    sized_box.state_change(|w| w.size).subscribe(move |size| {
      *c_changed_size.borrow_mut() = size.after;
    });

    let mut state = unsafe { sized_box.state_ref() };
    let mut wnd = Window::without_render(sized_box.box_it(), Size::new(500., 500.));
    wnd.render_ready();

    assert_eq!(*notified_count.borrow(), 0);
    assert_eq!(wnd.context().is_dirty(), false);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));
    {
      state.size = Size::new(1., 1.);
    }
    wnd.context.state_change_dispatch();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(wnd.context.is_dirty(), true);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    #[derive(Debug)]
    struct TestWidget;

    impl Compose for TestWidget {
      fn compose(&self, _: &mut BuildCtx) -> BoxedWidget {
        SizedBox { size: Size::new(100., 100.) }
          .into_stateful()
          .box_it()
      }
    }

    let mut wnd = Window::without_render(TestWidget.box_it(), Size::new(500., 500.));
    wnd.render_ready();
    let tree = &wnd.context().widget_tree;
    assert_eq!(tree.root().descendants(tree).count(), 2);
  }

  #[test]
  fn assigned_id_after_add_in_widget() {
    let w = SizedBox { size: Size::zero() }.into_stateful();
    let state_ref = unsafe { w.silent_ref() };

    let mut wnd = Window::without_render(w.box_it(), Size::new(500., 500.));

    let state_attr = state_ref.find_attr::<StateAttr>();
    assert!(state_attr.is_some());
    assert!(state_attr.unwrap().tree_info.is_some());

    // keep window live longer than `state_ref`
    wnd.render_ready();
  }

  #[test]
  fn state_ref_record() {
    let w = SizedBox { size: Size::zero() }.into_stateful();
    let mut silent_ref = unsafe { w.silent_ref() };
    let mut state_ref = unsafe { w.state_ref() };
    let mut tree = WidgetTree::new(widget_tree::WidgetNode::Render(Box::new(StatefulWrap(w))));

    {
      let _ = &mut state_ref.size;
      let (_, silent) = tree.pop_changed_widgets().unwrap();
      assert_eq!(silent, WidgetChangeFlags::ALL);
      assert!(tree.pop_changed_widgets().is_none());
    }

    {
      let _ = &mut silent_ref.size;
      let (_, silent) = tree.pop_changed_widgets().unwrap();
      assert_eq!(silent, WidgetChangeFlags::DIFFUSE);
      assert!(tree.pop_changed_widgets().is_none());
    }
  }
}
