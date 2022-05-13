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
use rxrust::prelude::*;
use std::{
  cell::{RefCell, RefMut},
  rc::Rc,
};

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
pub struct StateRef<'a, W> {
  inner_ref: RefMut<'a, W>,
  guard: StateRefGuard,
}

/// A reference of stateful widget, tracked the state change across if user
/// mutable reference the `SilentRef`. If mutable reference occur, state change
/// notify will trigger, but not effect the inner widget relayout or rebuild.
///
/// If you not very clear how `SilentRef` work, use [`StateRef`]! instead of.
pub struct SilentRef<W> {
  inner_ref: W,
  guard: SilentRefGuard,
}

/// The stateful widget generic implementation.
pub struct Stateful<W> {
  widget: Rc<RefCell<W>>,
  change_notifier: ChangeNotifier,
}

#[derive(Default, Clone)]
pub(crate) struct ChangeNotifier(Rc<RefCell<LocalSubject<'static, bool, ()>>>);

struct StateRefGuard {
  accessed: bool,
  notifier: ChangeNotifier,
}

struct SilentRefGuard {
  accessed: bool,
  notifier: ChangeNotifier,
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
  pub fn state_ref(&self) -> StateRef<W> {
    StateRef {
      inner_ref: self.widget.borrow_mut(),
      guard: StateRefGuard {
        accessed: false,
        notifier: self.change_notifier.clone(),
      },
    }
  }

  /// Return a `SilentMut` of the stateful widget.
  #[inline]
  pub fn silent_ref(&self) -> SilentRef<RefMut<W>> {
    SilentRef {
      inner_ref: self.widget.borrow_mut(),
      guard: SilentRefGuard {
        accessed: false,
        notifier: self.change_notifier.clone(),
      },
    }
  }

  /// Return a shallow reference to the stateful widget which modify the widget
  /// and not notify state change.
  #[inline]
  pub fn shallow_ref(&self) -> RefMut<W> { self.widget.borrow_mut() }

  /// Notify when this widget be mutable accessed, no mather if the widget
  /// really be modified, the value is hint if it's only access by silent ref.
  #[inline]
  pub fn change_stream(&self) -> LocalSubject<'static, bool, ()> {
    self.change_notifier.0.borrow().clone()
  }

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
  pub fn silent(&mut self) -> SilentRef<&mut RefMut<'a, W>> {
    SilentRef {
      inner_ref: &mut self.inner_ref,
      guard: SilentRefGuard {
        accessed: false,
        notifier: self.guard.notifier.clone(),
      },
    }
  }

  #[inline]
  pub fn shallow(&mut self) -> &mut RefMut<'a, W> { &mut self.inner_ref }
}

impl<W: std::ops::Deref> std::ops::Deref for SilentRef<W> {
  type Target = W::Target;

  #[inline]
  fn deref(&self) -> &Self::Target { self.inner_ref.deref() }
}

impl<W: std::ops::DerefMut> std::ops::DerefMut for SilentRef<W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.guard.accessed = true;
    self.inner_ref.deref_mut()
  }
}

impl<'a, W> std::ops::Deref for StateRef<'a, W> {
  type Target = W;

  #[inline]
  fn deref(&self) -> &Self::Target { self.inner_ref.deref() }
}

impl<'a, W> std::ops::DerefMut for StateRef<'a, W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.guard.accessed = true;
    self.inner_ref.deref_mut()
  }
}

impl<W> SingleChildWidget for Stateful<W> where W: SingleChildWidget {}

impl<W> MultiChildWidget for Stateful<W> where W: MultiChildWidget {}

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

impl Drop for StateRefGuard {
  fn drop(&mut self) {
    if self.accessed {
      self.notifier.0.borrow_mut().next(false)
    }
  }
}

impl Drop for SilentRefGuard {
  fn drop(&mut self) {
    if self.accessed {
      self.notifier.0.borrow_mut().next(true)
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
      declare Text {
        text: "Hello",
        style: TextStyle::default(),
        key: 1,
      }
    };

    let ctx = Context::new(stateful, 1., None);
    let tree = &ctx.widget_tree;
    let key = tree
      .root()
      .assert_get(tree)
      .query_first_type::<Key>(QueryOrder::InnerFirst);
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

    let mut state = sized_box.state_ref();
    let mut wnd = Window::without_render(sized_box.box_it(), Size::new(500., 500.));
    wnd.render_ready();

    assert_eq!(*notified_count.borrow(), 0);
    assert_eq!(wnd.context().is_dirty(), false);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));
    {
      state.size = Size::new(1., 1.);
    }
    wnd.context.tree_repair();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(wnd.context.is_dirty(), true);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut wnd = Window::without_render(
      widget! { declare SizedBox { size: Size::new(100., 100.) } },
      Size::new(500., 500.),
    );
    wnd.render_ready();
    let tree = &wnd.context().widget_tree;
    assert_eq!(tree.root().descendants(tree).count(), 2);
  }

  #[test]
  fn change_notify() {
    let notified = Rc::new(RefCell::new(vec![]));
    let w = SizedBox { size: Size::zero() }.into_stateful();
    w.change_stream()
      .subscribe(|b| notified.borrow_mut().push(b));

    {
      let _ = &mut w.state_ref().size;
    }
    assert_eq!(notified.borrow().deref(), &[false]);

    {
      let _ = &mut w.silent_ref().size;
    }
    assert_eq!(notified.borrow().deref(), &[false, true]);

    {
      let state_ref = w.state_ref();
      let silent_ref = w.silent_ref();
      &mut state_ref;
      &mut state_ref;
      &mut silent_ref;
      &mut silent_ref;
    }
    assert_eq!(notified.borrow().deref(), &[false, true, false, true]);
  }
}
