//! ## Stateless and Stateful
//! As default, In Ribir, every widget is stateless, just present like what you
//! declare and no interactive. That mean you can't modify the data of the
//! widget, the presentation of this widget is static.

//! But Ribir provide a stateful implementation version widget for every widget,
//! convert widget across [`IntoStateful`]!. So, in most cases you implement
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
//!       declare MockBox {
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
//! rect.tap(move |_| { state_ref.color = Color::BLACK; });
//! ```
//! In the above example, we implement a widget `Rectangle`, and use it to
//! change its color when user tapped.
//!
//! How to do if the `tap` behavior should as a part of the rectangle
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
//!       declare MockBox {
//!         size: this.size,
//!         background: this.color.clone(),
//!         tap: move |_| this_ref.color = Color::BLACK
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

use crate::{impl_proxy_query, impl_query_self_only, prelude::*};
use rxrust::{ops::box_it::LocalCloneBoxOp, prelude::*};
use std::{
  cell::{RefCell, RefMut, UnsafeCell},
  rc::Rc,
};

/// Convert a stateless widget to stateful which can provide a `StateRefCell`
/// to use to modify the states of the widget.
pub trait IntoStateful {
  fn into_stateful(self) -> Stateful<Self>
  where
    Self: Sized;
}

/// The stateful widget generic implementation.
pub struct Stateful<W> {
  pub(crate) widget: Rc<RefCell<W>>,
  pub(crate) change_notifier: StateChangeNotifier,
}

/// notify downstream when widget state changed, the value mean if the change it
/// as silent or not.
#[derive(Default, Clone)]
pub(crate) struct StateChangeNotifier(LocalSubject<'static, ChangeScope, ()>);

/// A reference of `Stateful which tracked the state change across if user
/// mutable deref this reference.
///
/// `StateRef` also help implicit borrow inner widget mutable or not by deref or
/// deref_mut. And early drop the inner borrow if need, so the borrow lifetime
/// not bind to struct lifetime. Useful to avoid borrow conflict. For example
///
/// ```ignore
/// (this.ref_y() > 0).then(move || this.mut_y());
/// ```
///
/// Assume above code in `widget!` macro and `this` is a tracked stateful
/// widget, Two variables of borrow result of two `this` have lifetime overlap.
/// But in logic, first borrow needn't live as long as the statement. See
/// relative rust issue https://github.com/rust-lang/rust/issues/37612

pub struct StateRef<'a, W> {
  widget: &'a Stateful<W>,
  current_ref: UnsafeCell<Option<RefMut<'a, W>>>,
  mut_accessed: Option<ChangeScope>,
  current_scope: ChangeScope,
}

bitflags! {
  pub struct ChangeScope: u8 {
    /// state change only effect the data, transparent to ribir framework.
    const DATA  = 0x001;
    /// state change only effect to framework, transparent to widget data.
    const FRAMEWORK = 0x010;
    /// state change effect both widget data and framework.
    const BOTH = Self::DATA.bits | Self::FRAMEWORK.bits;
  }
}

#[derive(Clone)]
pub struct StateChange<T: Clone> {
  pub before: T,
  pub after: T,
}

impl<W> Clone for Stateful<W> {
  #[inline]
  fn clone(&self) -> Self {
    Self {
      widget: self.widget.clone(),
      change_notifier: self.change_notifier.clone(),
    }
  }
}

impl<W> Stateful<W> {
  pub fn new(widget: W) -> Self {
    Stateful {
      widget: Rc::new(RefCell::new(widget)),
      change_notifier: <_>::default(),
    }
  }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// data and framework.
  #[inline]
  pub fn state_ref(&self) -> StateRef<W> { StateRef::new(self, ChangeScope::BOTH) }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// data only, the relayout or paint depends on this object will not be skip.
  ///
  /// If you not very clear how `silent_ref` work, use [`Stateful::state_ref`]!
  /// instead of.
  #[inline]
  pub fn silent_ref(&self) -> StateRef<W> { StateRef::new(self, ChangeScope::DATA) }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// framework only. That means this modify only effect framework but not
  /// effect on data. And usually use it to temporary to modify the `Stateful`.
  ///
  /// If you not very clear how `shallow_ref` work, use [`Stateful::state_ref`]!
  /// instead of.
  #[inline]
  pub fn shallow_ref(&self) -> StateRef<W> { StateRef::new(self, ChangeScope::FRAMEWORK) }

  /// Directly mutable borrow the inner widget and control on it, nothing will
  /// be know by framework, use it only if you know how the four kind of ref
  /// (state, silent, shallow, raw) of stateful widget work.
  #[inline]
  pub fn raw_ref(&self) -> RefMut<W> { self.widget.borrow_mut() }

  /// Notify when this widget be mutable accessed, no mather if the widget
  /// really be modified, the value is hint if it's only access by silent ref.
  #[inline]
  pub fn change_stream(&self) -> LocalCloneBoxOp<'static, (), ()> {
    self
      .raw_change_stream()
      .filter_map(|s: ChangeScope| s.contains(ChangeScope::DATA).then(|| ()))
      .box_it()
  }

  /// Pick field change stream from the widget change
  pub fn state_change<T: Clone + 'static>(
    &self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()>
  where
    Self: 'static,
  {
    let v = pick(&self.state_ref());
    let init = StateChange { before: v.clone(), after: v };
    let stateful = self.clone();
    self.change_stream().scan_initial(init, move |mut init, _| {
      init.before = init.after;
      init.after = pick(&stateful.state_ref());
      init
    })
  }

  pub fn raw_change_stream(&self) -> LocalSubject<'static, ChangeScope, ()> {
    self.change_notifier.0.clone()
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  #[inline]
  pub fn clone_stateful(&self) -> Stateful<W> { self.clone() }
}

impl<T: Clone + std::cmp::PartialEq> StateChange<T> {
  #[inline]
  pub fn is_same(&self) -> bool { self.after == self.before }

  #[inline]
  pub fn not_same(&self) -> bool { self.before != self.after }
}

impl<'a, W> std::ops::Deref for StateRef<'a, W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    // SAFETY: `RefCell` guarantees unique access, and `InnerRef` not thread safe no
    // data race occur to fill the option value at same time.
    let inner = unsafe { &mut *self.current_ref.get() };
    inner.get_or_insert_with(|| self.widget.widget.borrow_mut())
  }
}

impl<'a, W> std::ops::DerefMut for StateRef<'a, W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: `RefCell` guarantees unique access.
    let inner = unsafe { &mut *self.current_ref.get() };
    if inner.is_none() {
      *inner = Some(self.widget.widget.borrow_mut())
    }
    if let Some(mut_access) = self.mut_accessed {
      self.mut_accessed = Some(mut_access | self.current_scope)
    } else {
      self.mut_accessed = Some(self.current_scope)
    }
    inner.get_or_insert_with(|| self.widget.widget.borrow_mut())
  }
}

impl<'a, W> StateRef<'a, W> {
  /// Fork a silent reference
  pub fn silent(&mut self) -> StateRef<'a, W> {
    self.release_current_borrow();
    StateRef::new(self.widget, ChangeScope::DATA)
  }

  #[inline]
  pub fn shallow(&mut self) -> StateRef<'a, W> {
    self.release_current_borrow();
    StateRef::new(self.widget, ChangeScope::FRAMEWORK)
  }

  fn new(widget: &'a Stateful<W>, scope: ChangeScope) -> Self {
    Self {
      widget,
      current_ref: UnsafeCell::new(None),
      mut_accessed: None,
      current_scope: scope,
    }
  }
  #[inline]
  pub fn release_current_borrow(&mut self) { self.current_ref.get_mut().take(); }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  // todo: clone stateful mutable access across a modify event is incorrect.
  #[inline]
  pub fn clone_stateful(&mut self) -> Stateful<W> {
    self.release_current_borrow();
    self.widget.clone()
  }
}

impl<W: SingleChild> SingleChild for Stateful<W> {}

impl<W: MultiChild> MultiChild for Stateful<W> {}

impl<W: Render + 'static> Render for Stateful<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.widget.borrow().perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.widget.borrow().only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.widget.borrow().paint(ctx) }

  #[inline]
  fn can_overflow(&self) -> bool { self.widget.borrow().can_overflow() }

  #[inline]
  fn hit_test(&self, ctx: &TreeCtx, pos: Point) -> HitTest {
    self.widget.borrow().hit_test(ctx, pos)
  }
}

impl<W: Compose> Compose for Stateful<W> {
  fn compose(this: StateWidget<Self>) -> Widget {
    let w = match this {
      StateWidget::Stateless(s) => s,
      StateWidget::Stateful(s) => s.widget.borrow().clone(),
    };
    Compose::compose(StateWidget::Stateful(w))
  }
}

impl<W: ComposeChild> ComposeChild for Stateful<W> {
  type Child = W::Child;

  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let w = match this {
      StateWidget::Stateless(s) => s,
      StateWidget::Stateful(s) => s.widget.borrow().clone(),
    };
    ComposeChild::compose_child(StateWidget::Stateful(w), child)
  }
}

impl<'a, W> Drop for StateRef<'a, W> {
  fn drop(&mut self) {
    if let Some(scope) = self.mut_accessed {
      self.release_current_borrow();
      self.widget.raw_change_stream().next(scope)
    }
  }
}

// Implement IntoStateful for all widget
impl<W> IntoStateful for W {
  #[inline]
  fn into_stateful(self) -> Stateful<W> { Stateful::new(self) }
}

impl<W: Query + 'static> Query for Stateful<W> {
  impl_proxy_query!(self.change_notifier, self.widget);
}

impl Query for StateChangeNotifier {
  impl_query_self_only!();
}

impl StateChangeNotifier {
  pub(crate) fn change_stream(&self) -> LocalSubject<'static, ChangeScope, ()> { self.0.clone() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    // Simulate `MockBox` widget need modify its size in event callback. Can use the
    // `cell_ref` in closure.
    let stateful = MockBox { size: Size::zero() }.into_stateful();
    {
      stateful.state_ref().size = Size::new(100., 100.)
    }
    assert_eq!(stateful.state_ref().size, Size::new(100., 100.));
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let sized_box = MockBox { size: Size::new(100., 100.) }.into_stateful();
    sized_box
      .change_stream()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    sized_box.state_change(|w| w.size).subscribe(move |size| {
      *c_changed_size.borrow_mut() = size.after;
    });

    let state = sized_box.clone();
    let mut wnd = Window::default_mock(sized_box.into_widget(), None);
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 0);
    assert_eq!(wnd.widget_tree.is_dirty(), false);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));

    {
      state.state_ref().size = Size::new(1., 1.);
    }
    assert_eq!(wnd.widget_tree.is_dirty(), true);
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut wnd = Window::default_mock(widget! { MockBox { size: Size::new(100., 100.) } }, None);
    wnd.draw_frame();
    let tree = &wnd.widget_tree;
    assert_eq!(tree.root().descendants(tree).count(), 1);
  }

  #[test]
  fn change_notify() {
    let notified = Rc::new(RefCell::new(vec![]));
    let c_notified = notified.clone();
    let w = MockBox { size: Size::zero() }.into_stateful();
    w.raw_change_stream()
      .subscribe(move |b| c_notified.borrow_mut().push(b));

    {
      let _ = &mut w.state_ref().size;
    }
    assert_eq!(notified.borrow().deref(), &[ChangeScope::BOTH]);

    {
      let _ = &mut w.silent_ref().size;
    }
    assert_eq!(
      notified.borrow().deref(),
      &[ChangeScope::BOTH, ChangeScope::DATA]
    );

    {
      let mut state_ref = w.state_ref();
      let mut silent_ref = w.silent_ref();
      let _ = &mut state_ref;
      let _ = &mut state_ref;
      let _ = &mut silent_ref;
      let _ = &mut silent_ref;
    }
    assert_eq!(
      notified.borrow().deref(),
      &[ChangeScope::BOTH, ChangeScope::DATA]
    );
  }
}
