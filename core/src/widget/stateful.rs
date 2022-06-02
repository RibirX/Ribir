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

use crate::prelude::*;
use lazy_static::__Deref;
use rxrust::prelude::*;
use std::{
  cell::{RefCell, RefMut, UnsafeCell},
  ops::DerefMut,
  rc::Rc,
};

use super::ComposedWidget;

/// Convert a stateless widget to stateful which can provide a `StateRefCell`
/// to use to modify the states of the widget.
pub trait IntoStateful {
  fn into_stateful(self) -> Stateful<Self>
  where
    Self: Sized;
}

/// A reference of stateful widget, can use it to directly access and modify
/// stateful widget. Tracked the state change across if user mutable reference
/// the `StateRef` and trigger state change notify and require `ribir` to
/// rebuild or relayout inner widget.
pub struct StateRef<'a, W>(InnerRef<'a, W>);

/// A reference of stateful widget, tracked the state change across if user
/// mutable reference the `SilentRef`. If mutable reference occur, state change
/// notify will trigger, but not effect the inner widget relayout or rebuild.
///
/// If you not very clear how `SilentRef` work, use [`StateRef`]! instead of.
pub struct SilentRef<'a, W>(InnerRef<'a, W>);

/// The stateful widget generic implementation.
pub struct Stateful<W> {
  pub(crate) widget: Rc<RefCell<W>>,
  pub(crate) change_notifier: StateChangeNotifier,
}

/// notify downstream when widget state changed, the value mean if the change it
/// as silent or not.
#[derive(Default, Clone)]
pub(crate) struct StateChangeNotifier(LocalSubject<'static, bool, ()>);

/// `InnerRef` help implicit borrow inner widget mutable or not by deref or
/// deref_mut. And early drop the inner borrow if need, so the borrow lifetime
/// not bind to struct lifetime. Useful to avoid borrow conflict. For example
///
/// ```ignore
/// (this.xxx() > 0).then(move || this.mut_y());
/// ```
///
/// Assume above code in `widget!` macro and `this` is a tracked stateful
/// widget, Two variables of borrow result of two `this` have lifetime overlap.
/// But in logic, first borrow needn't live as long as the statement. See
/// relative rust issue https://github.com/rust-lang/rust/issues/37612

struct InnerRef<'a, W> {
  widget: &'a Stateful<W>,
  current_ref: UnsafeCell<Option<RefMut<'a, W>>>,
  mut_accessed: bool,
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
  // Convert a widget to a stateful widget, only called by framework. Maybe you
  // want [`into_stateful`](IntoStateful::into_stateful)
  pub(crate) fn new(widget: W) -> Self {
    Stateful {
      widget: Rc::new(RefCell::new(widget)),
      change_notifier: <_>::default(),
    }
  }

  /// Return a `StateRef` of the stateful widget.
  #[inline]
  pub fn state_ref(&self) -> StateRef<W> { StateRef(InnerRef::new(self)) }

  /// Return a `SilentMut` of the stateful widget.
  #[inline]
  pub fn silent_ref(&self) -> SilentRef<W> { SilentRef(InnerRef::new(self)) }

  /// Return a shallow reference to the stateful widget which modify the widget
  /// and not notify state change.
  #[inline]
  pub fn shallow_ref(&self) -> RefMut<W> { self.widget.borrow_mut() }

  /// Notify when this widget be mutable accessed, no mather if the widget
  /// really be modified, the value is hint if it's only access by silent ref.
  #[inline]
  pub fn change_stream(&self) -> LocalSubject<'static, bool, ()> { self.change_notifier.0.clone() }

  /// Pick field change stream from the widget change

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
}

impl<'a, W> StateRef<'a, W> {
  /// Fork a silent reference
  pub fn silent(&mut self) -> SilentRef<'a, W> {
    self.0.release_current_borrow();
    SilentRef(InnerRef::new(self.0.widget))
  }

  #[inline]
  pub fn shallow(&mut self) -> RefMut<'a, W> {
    self.0.release_current_borrow();
    self.0.widget.widget.borrow_mut()
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  #[inline]
  pub fn clone(&self) -> Stateful<W> {
    // todo
    // self.0.release_current_borrow();
    self.0.widget.clone()
  }

  #[inline]
  pub fn state_ref(self) -> Self { self }
}

impl<'a, W> std::ops::Deref for SilentRef<'a, W> {
  type Target = W;

  #[inline]
  fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<'a, W> std::ops::DerefMut for SilentRef<'a, W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0.deref_mut() }
}

impl<'a, W> std::ops::Deref for StateRef<'a, W> {
  type Target = W;

  #[inline]
  fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<'a, W> std::ops::DerefMut for StateRef<'a, W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0.deref_mut() }
}

impl<'a, W> std::ops::Deref for InnerRef<'a, W> {
  type Target = W;

  fn deref(&self) -> &Self::Target {
    // SAFETY: `RefCell` guarantees unique access, and `InnerRef` not thread safe no
    // data race occur to fill the option value at same time.
    let inner = unsafe { &mut *self.current_ref.get() };
    if inner.is_none() {
      *inner = Some(self.widget.widget.borrow_mut())
    }
    inner.as_ref().unwrap().deref()
  }
}

impl<'a, W> std::ops::DerefMut for InnerRef<'a, W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: `RefCell` guarantees unique access.
    let inner = unsafe { &mut *self.current_ref.get() };
    if inner.is_none() {
      *inner = Some(self.widget.widget.borrow_mut())
    }
    self.mut_accessed = true;
    inner.as_mut().unwrap().deref_mut()
  }
}

impl<'a, W> InnerRef<'a, W> {
  fn new(widget: &'a Stateful<W>) -> Self {
    Self {
      widget,
      current_ref: UnsafeCell::new(None),
      mut_accessed: false,
    }
  }

  #[inline]
  fn release_current_borrow(&mut self) { self.current_ref.get_mut().take(); }
}

impl<W: Render> Render for Stateful<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.state_ref().perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.state_ref().only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.state_ref().paint(ctx) }
}

impl<W: SingleChild> SingleChild for Stateful<W> {}

impl<W: MultiChild> MultiChild for Stateful<W> {}

impl<C: Compose + 'static> IntoWidget<&dyn Compose> for Stateful<C> {
  #[inline]
  fn into_widget(self) -> Widget {
    Widget(WidgetInner::Compose(Box::new(|ctx| {
      ComposedWidget {
        composed: Compose::compose(self, ctx),
        by: PhantomData::<C>,
      }
      .into_widget()
    })))
  }
}

impl<'a, W> Drop for StateRef<'a, W> {
  fn drop(&mut self) {
    if self.0.mut_accessed {
      self.0.widget.change_stream().next(false)
    }
  }
}

impl<'a, W> Drop for SilentRef<'a, W> {
  fn drop(&mut self) {
    if self.0.mut_accessed {
      self.0.widget.change_stream().next(true)
    }
  }
}

// Implement IntoStateful for all widget
impl<W> IntoStateful for W
where
  W: WidgetMarker,
{
  #[inline]
  fn into_stateful(self) -> Stateful<W> { Stateful::new(self) }
}

impl<W: 'static> QueryType for Stateful<W> {
  fn query_all(
    &self,
    type_id: std::any::TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
    order: QueryOrder,
  ) {
    let w = self.widget.borrow();
    let widget = w.deref();
    let mut continue_query = true;
    match order {
      QueryOrder::InnerFirst => {
        widget.query_all(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          if let Some(a) = self.change_notifier.query(type_id) {
            callback(a);
          }
        }
      }
      QueryOrder::OutsideFirst => {
        if let Some(a) = self.change_notifier.query(type_id) {
          continue_query = callback(a);
        }
        if continue_query {
          widget.query_all(type_id, callback, order);
        }
      }
    }
  }

  fn query_all_mut(
    &mut self,
    type_id: std::any::TypeId,
    callback: &mut dyn FnMut(&mut dyn Any) -> bool,
    order: QueryOrder,
  ) {
    let mut continue_query = true;
    let mut w = self.widget.borrow_mut();
    let widget = w.deref_mut();
    match order {
      QueryOrder::InnerFirst => {
        widget.query_all_mut(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          if let Some(a) = self.change_notifier.query_mut(type_id) {
            callback(a);
          }
        }
      }
      QueryOrder::OutsideFirst => {
        if let Some(a) = self.change_notifier.query_mut(type_id) {
          continue_query = callback(a);
        }
        if continue_query {
          widget.query_all_mut(type_id, callback, order);
        }
      }
    }
  }
}

impl StateChangeNotifier {
  pub(crate) fn change_stream(&self) -> LocalSubject<'static, bool, ()> { self.0.clone() }
}

#[cfg(test)]
mod tests {
  use lazy_static::__Deref;

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
      stateful.state_ref().text = "World!".into();
    }
    assert_eq!(&*stateful.state_ref().text, "World!");
  }

  #[test]
  fn stateful_id_check() {
    let stateful = widget! {
      Text {
        text: "Hello",
        style: TextStyle::default(),
        key: 1,
      }
    };

    let ctx = Context::new(stateful, 1., None);
    let tree = &ctx.widget_tree;
    let mut key = None;
    tree
      .root()
      .assert_get(tree)
      .query_on_first_type(QueryOrder::InnerFirst, |k: &Key| key = Some(k.clone()));
    assert!(key.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let sized_box = SizedBox { size: Size::new(100., 100.) }.into_stateful();
    sized_box
      .change_stream()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    sized_box.state_change(|w| w.size).subscribe(move |size| {
      *c_changed_size.borrow_mut() = size.after;
    });

    let state = sized_box.clone();
    let mut wnd = Window::without_render(sized_box.into_widget(), Size::new(500., 500.));
    wnd.render_ready();

    assert_eq!(*notified_count.borrow(), 0);
    assert_eq!(wnd.context().is_dirty(), false);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));
    {
      state.state_ref().size = Size::new(1., 1.);
    }
    wnd.context.tree_repair();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(wnd.context.is_dirty(), true);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut wnd = Window::without_render(
      widget! { SizedBox { size: Size::new(100., 100.) } },
      Size::new(500., 500.),
    );
    wnd.render_ready();
    let tree = &wnd.context().widget_tree;
    assert_eq!(tree.root().descendants(tree).count(), 2);
  }

  #[test]
  fn change_notify() {
    let notified = Rc::new(RefCell::new(vec![]));
    let c_notified = notified.clone();
    let w = SizedBox { size: Size::zero() }.into_stateful();
    w.change_stream()
      .subscribe(move |b| c_notified.borrow_mut().push(b));

    {
      let _ = &mut w.state_ref();
    }
    assert_eq!(notified.borrow().deref(), &[false]);

    {
      let _ = &mut w.silent_ref().size;
    }
    assert_eq!(notified.borrow().deref(), &[false, true]);

    {
      let mut state_ref = w.state_ref();
      let mut silent_ref = w.silent_ref();
      let _ = &mut state_ref;
      let _ = &mut state_ref;
      let _ = &mut silent_ref;
      let _ = &mut silent_ref;
    }
    assert_eq!(notified.borrow().deref(), &[false, true, false, true]);
  }
}
