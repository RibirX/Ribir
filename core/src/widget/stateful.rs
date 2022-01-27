//! ## Stateless and Stateful
//! As default, In Ribir, every widget is stateless, just present like what you
//! declare and no interactive. That mean you can't modify the data of the
//! widget, the presentation of this widget is static.

//! But Ribir provide a common method to convert a widget from sateless to
//! stateful if a widget need repaint or relayout to respond to some widget
//! change. This depends on [`Stateful`][Stateful] and
//! [`IntoStateful`][IntoStateful]
//! Use the `#[stateful]` attr  to provide a stateful version widget named
//! `StatefulXXX` which just a tuple struct wrap the
//! [`StatefulImpl`][StatefulImpl] with the stateless version and implement
//! [`IntoStateful`][IntoStateful]  for the stateless version widget. We
//! needn't write any logic code to support stateful.

//! # Example
//! This example implement a rectangle widget which support change its size and
//! fill color.
//! ```
//! # #![feature(trivial_bounds, negative_impls)]
//! # use ribir::prelude::*;
//!
//! #[stateful]
//! struct Rectangle {
//!   size: Size,
//!   color: Color,
//! }
//!
//! impl CombinationWidget for Rectangle {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     declare!{
//!       SizedBox {
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
//! let mut state_ref = rect.state_ref();
//! rect.on_tap(move |_| {
//!   state_ref.color = Color::BLACK;
//! });
//! ```
//! In the above example, we implement a widget `Rectangle`, and use it to
//! change its color when user tapped.
//!
//! How to do if we want this behavior as a part of the rectangle itself. In
//! other word, a stateless `Rectangle` is useless, we only need a stateful
//! `Rectangle`. To implement it, we can specify `custom` meta to
//! `#[stateful(custom)]` attr. This tell Ribir, "I want to implement
//! RenderWidget/CombinationWidget for the stateful widget by myself instead of
//! direct derive from the stateless version.
//! ```
//! # #![feature(trivial_bounds, negative_impls)]
//! # use ribir::prelude::*;
//!
//! #[stateful(custom)]
//! struct Rectangle {
//!   size: Size,
//!   color: Color,
//! }
//!
//! impl CombinationWidget for StatefulRectangle {
//!   fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
//!     let mut state_ref = self.state_ref();
//!     declare!{
//!       SizedBox {
//!         size: self.size,
//!         background: self.color.clone(),
//!         on_tap: move |_| state_ref.color = Color::BLACK
//!       }
//!     }
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
use std::{ptr::NonNull, rc::Rc};

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
pub struct StateRef<W>(SilentRef<W>);

/// A reference of stateful widget, tracked the state change across if user
/// mutable reference the `SilentRef`. If mutable reference occur, state change
/// notify will trigger, but not effect the inner widget relayout or rebuild.
///
/// If you not very clear how `SilentRef` work, use [`StateRef`]! instead of.
pub struct SilentRef<W>(Stateful<W>);

/// The stateful widget generic implementation.
pub struct Stateful<W> {
  widget: Rc<AttrWidget<W>>,
  state_attr: NonNull<StateAttr>,
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

#[derive(Default)]
pub(crate) struct StateAttr {
  pub(crate) tree_info: Option<TreeInfo>,
  subject: Option<LocalSubject<'static, (), ()>>,
}

impl<W: 'static> Clone for StateRef<W> {
  fn clone(&self) -> Self { self.0.state_ref() }
}

impl<W: 'static> Stateful<W> {
  // Convert a widget to a stateful widget, only called by framework. Maybe you
  // want [`into_stateful`](IntoStateful::into_stateful)
  pub fn new(w: W) -> Self
  where
    W: Widget,
  {
    let mut attrs: Attributes = <_>::default();
    attrs.insert(StateAttr::default());
    Stateful {
      state_attr: NonNull::new(attrs.find_mut::<StateAttr>().unwrap() as *mut _).unwrap(),
      widget: Rc::new(AttrWidget { widget: w, attrs }),
    }
  }

  /// Return a `StateRef` of the stateful widget, caller should careful not keep
  /// it live not longer than its widget.
  #[inline]
  pub unsafe fn state_ref(&self) -> StateRef<W> { StateRef(self.silent_ref()) }

  /// Return a `SilentRef` of the stateful widget. Caller should careful not
  /// keep it live not longer than its widget.
  #[inline]
  pub unsafe fn silent_ref(&self) -> SilentRef<W> { SilentRef(NonNull::from(&*self.0)) }

  /// Event emitted when this widget modified. No mather if the widget really
  pub fn change_stream(&mut self) -> LocalSubject<'static, (), ()> {
    unsafe { self.state_attr.as_mut().state_subject() }
  }

  /// Pick field change stream from the widget change
  pub fn state_change<T: Clone + 'static>(
    &mut self,
    pick: impl Fn(&W) -> T + 'static,
  ) -> impl LocalObservable<'static, Item = StateChange<T>, Err = ()> {
    let state_ref = unsafe { self.state_ref() };
    let v = pick(&self.widget);
    let init = StateChange { before: v.clone(), after: v };
    self.change_stream().scan_initial(init, move |mut init, _| {
      init.before = init.after;
      init.after = pick(&state_ref);
      init
    })
  }
}

impl<W> StateRef<W> {
  // convert a `StateRef` to `SilentRef`
  pub fn silent(&self) -> SilentRef<W> {
    SilentRef(Stateful {
      widget: self.widget.clone(),
      state_attr: self.state_attr.clone(),
    })
  }
}

impl<W> std::ops::Deref for Stateful<W> {
  type Target = AttrWidget<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { self.widget.as_ref() }
}

impl<W> std::ops::DerefMut for Stateful<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // Safety
    // - `Stateful` is not support clone, so as a widget it's unique and safe to get
    //   inner mutable reference。
    // - StateRef may hold a refcount of this, in the `declare!` macro, ribir
    //   guarantee the generate code have not data race.
    // - User directly use the internal api should be careful not hold the inner
    //   mutable reference.
    unsafe { Rc::get_mut_unchecked(&mut self.widget) }
  }
}

impl<W> std::ops::Deref for SilentRef<W> {
  type Target = Stateful<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<W> std::ops::DerefMut for SilentRef<W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    // todo: notify change on drop
    // Safety: the back data of `state_attr` pointer have the same lifetime with
    // this pointer. And the pointer is use only in deref_mut in StateRef or
    // SilentRef
    let state_attr = unsafe { self.0.state_attr.as_mut() };
    if let Some(subject) = &mut state_attr.subject {
      subject.next(());
    }

    &mut self.0
  }
}

impl<W> std::ops::Deref for StateRef<W> {
  type Target = Stateful<W>;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<W> std::ops::DerefMut for StateRef<W> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // todo: notify change on drop
    // Safety: the back data of `state_attr` pointer have the same lifetime with
    // this pointer. And the pointer is use only in deref_mut in StateRef or
    // SilentRef

    let state_attr = unsafe { self.0.state_attr.as_mut() };
    if let Some(TreeInfo { mut tree, id }) = state_attr.tree_info {
      let tree = unsafe { tree.as_mut() };
      id.mark_changed(tree);
    }
    &mut self.0
  }
}

impl StateAttr {
  pub(crate) fn assign_id(&mut self, id: WidgetId, tree: NonNull<WidgetTree>) {
    debug_assert!(self.tree_info.is_none());
    self.tree_info = Some(TreeInfo { tree, id })
  }

  fn state_subject(&mut self) -> LocalSubject<'static, (), ()> {
    self.subject.get_or_insert_with(<_>::default).clone()
  }
}

/// A wrap for `StatefulImpl` to help we can implement stateful version widget
/// for for all widget, and avoid trait implement conflict.
pub struct StatefulWrap<W>(Stateful<W>);

impl<W: RenderWidget> IntoRender for Stateful<W> {
  type R = StatefulWrap<W>;

  #[inline]
  fn into_render(self) -> Self::R { StatefulWrap(self) }
}

impl<W: CombinationWidget> IntoCombination for Stateful<W> {
  type C = StatefulWrap<W>;
  #[inline]
  fn into_combination(self) -> Self::C { StatefulWrap(self) }
}

impl<W> SingleChildWidget for StatefulWrap<W> where W: SingleChildWidget + RenderWidget {}

impl<W> MultiChildWidget for StatefulWrap<W> where W: MultiChildWidget + RenderWidget {}

impl<W: CombinationWidget> CombinationWidget for StatefulWrap<W> {
  #[inline]
  fn build(&self, ctx: BuildCtx<Self>) -> BoxedWidget { self.0.build(ctx.cast_type()) }
}

impl<W: RenderWidget> RenderWidget for StatefulWrap<W> {
  type RO = W::RO;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.0.create_render_object() }

  #[inline]
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    self.0.update_render_object(object, ctx)
  }
}

// Implement IntoStateful for all widget

impl<W: Widget + 'static> IntoStateful for W {
  type S = Stateful<W>;
  #[inline]
  fn into_stateful(self) -> Self::S { Stateful::new(self) }
}

impl<W> IntoStateful for Stateful<W> {
  type S = Self;
  #[inline]
  fn into_stateful(self) -> Self::S { self }
}

impl<W: IntoStateful> IntoStateful for AttrWidget<W> {
  type S = Stateful<W>;

  fn into_stateful(self) -> Self::S {
    let Self { widget, mut attrs } = self;
    attrs.insert(StateAttr::default());
    Stateful {
      state_attr: NonNull::new(attrs.find_mut::<StateAttr>().unwrap() as *mut _).unwrap(),
      widget: Rc::new(AttrWidget { widget, attrs }),
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
  fn downcast() {
    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());

    let stateful = Text {
      text: "Hello".into(),
      style: TextStyle::default(),
    }
    .into_stateful();
    // now key widget inherit from stateful widget.
    let key = stateful.with_key(1);
    let tree = unsafe { tree.as_mut().get_unchecked_mut() };
    let id = tree.set_root(key.box_it(), &mut render_tree);

    let key_back = id
      .get(tree)
      .and_then(|w| (w as &dyn BuiltinAttrs).get_key());
    assert!(key_back.is_some());
  }

  #[test]
  fn state_notify_and_relayout() {
    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let mut render_tree = render_tree::RenderTree::default();
    let mut tree = Box::pin(widget_tree::WidgetTree::default());
    let sized_box = SizedBox { size: Size::new(100., 100.) }.into_stateful();
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
    assert_eq!(tree.changed_widgets().len(), 0);
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));
    {
      state.size = Size::new(1., 1.);
    }
    tree.notify_state_change_until_empty();
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
        SizedBox { size: Size::new(100., 100.) }
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
