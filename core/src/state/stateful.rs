use crate::{impl_proxy_query, impl_query_self_only, prelude::*};
pub use guards::ModifyGuard;
use rxrust::{ops::box_it::BoxOp, prelude::*};
use std::{
  cell::{Cell, UnsafeCell},
  convert::Infallible,
  ops::{Deref, DerefMut},
  rc::Rc,
};

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  inner: Rc<InnerStateful<W>>,
  modify_notifier: StateChangeNotifier,
}

/// notify downstream when widget state changed, the value mean if the change it
/// as silent or not.
#[derive(Default, Clone)]
pub(crate) struct StateChangeNotifier(Subject<'static, ModifyScope, Infallible>);

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
  /// - None, Not used the value
  /// - Some(false), borrow used the value
  /// - Some(true), mutable borrow used the value
  mut_accessed_flag: Cell<Option<bool>>,
  modify_scope: ModifyScope,
  value: ModifyGuard<'a, W>,
}

bitflags! {
  pub struct ModifyScope: u8 {
    /// state change only effect the data, transparent to ribir framework.
    const DATA  = 0x001;
    /// state change only effect to framework, transparent to widget data.
    const FRAMEWORK = 0x010;
    /// state change effect both widget data and framework.
    const BOTH = Self::DATA.bits | Self::FRAMEWORK.bits;
  }
}

mod guards {
  use super::*;
  pub struct ModifyGuard<'a, W>(&'a Stateful<W>);

  impl<'a, W> ModifyGuard<'a, W> {
    pub(crate) fn new(data: &'a Stateful<W>) -> Self {
      let guards = &data.inner.guard_cnt;
      guards.set(guards.get() + 1);
      Self(data)
    }

    pub(crate) fn inner_ref(&self) -> &'a Stateful<W> { self.0 }
  }

  impl<'a, W> Drop for ModifyGuard<'a, W> {
    fn drop(&mut self) {
      let guards = &self.0.inner.guard_cnt;
      guards.set(guards.get() - 1);

      if guards.get() == 0 {
        let inner = &self.0.inner;
        assert_eq!(UNUSED, inner.borrow_flag.get());
        let scope = inner.modify_scope.take();
        if let Some(scope) = scope {
          self.0.raw_modifies().next(scope);
        }
      }
    }
  }

  impl<'a, W> std::ops::Deref for ModifyGuard<'a, W> {
    type Target = Stateful<W>;

    #[inline]
    fn deref(&self) -> &Self::Target { self.0 }
  }
}

type BorrowFlag = isize;
const UNUSED: BorrowFlag = 0;

#[inline(always)]
fn is_reading(x: BorrowFlag) -> bool { x > UNUSED }

struct InnerStateful<W> {
  borrow_flag: Cell<BorrowFlag>,
  modify_scope: Cell<Option<ModifyScope>>,
  #[cfg(debug_assertions)]
  borrowed_at: Cell<Option<&'static std::panic::Location<'static>>>,
  guard_cnt: Cell<usize>,
  data: UnsafeCell<W>,
}

impl<W> Clone for Stateful<W> {
  #[inline]
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      modify_notifier: self.modify_notifier.clone(),
    }
  }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Stateful {
      inner: Rc::new(InnerStateful {
        data: UnsafeCell::new(data),
        borrow_flag: Cell::new(0),
        modify_scope: Cell::new(None),
        #[cfg(debug_assertions)]
        borrowed_at: Cell::new(None),
        guard_cnt: Cell::new(0),
      }),
      modify_notifier: <_>::default(),
    }
  }

  /// Return a guard that batch the modify event.
  #[inline]
  pub fn modify_guard(&self) -> ModifyGuard<W> { ModifyGuard::new(self) }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// data and framework.
  #[inline]
  pub fn state_ref(&self) -> StateRef<W> { StateRef::new(self, ModifyScope::BOTH) }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// data only, the relayout or paint depends on this object will not be skip.
  ///
  /// If you not very clear how `silent_ref` work, use [`Stateful::state_ref`]!
  /// instead of.
  #[inline]
  pub fn silent_ref(&self) -> StateRef<W> { StateRef::new(self, ModifyScope::DATA) }

  /// Return a reference of `Stateful`, modify across this reference will notify
  /// framework only. That means this modify only effect framework but not
  /// effect on data. And usually use it to temporary to modify the `Stateful`.
  ///
  /// If you not very clear how `shallow_ref` work, use [`Stateful::state_ref`]!
  /// instead of.
  #[inline]
  pub(crate) fn shallow_ref(&self) -> StateRef<W> { StateRef::new(self, ModifyScope::FRAMEWORK) }

  pub fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.modify_notifier.raw_modifies()
  }

  /// Notify when this widget be mutable accessed, no mather if the widget
  /// really be modified, the value is hint if it's only access by silent ref.
  #[inline]
  pub fn modifies(&self) -> BoxOp<'static, (), Infallible> {
    self
      .raw_modifies()
      .filter_map(|s: ModifyScope| s.contains(ModifyScope::DATA).then_some(()))
      .box_it()
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  #[inline]
  pub fn clone_stateful(&self) -> Stateful<W> { self.clone() }

  pub(crate) fn try_into_inner(self) -> Result<W, Self> {
    if Rc::strong_count(&self.inner) == 1 {
      let inner = Rc::try_unwrap(self.inner).unwrap_or_else(|_| unreachable!());
      Ok(inner.data.into_inner())
    } else {
      Err(self)
    }
  }
}

macro_rules! debug_borrow_location {
  ($this: ident) => {
    #[cfg(debug_assertions)]
    {
      let caller = std::panic::Location::caller();
      $this.value.inner.borrowed_at.set(Some(caller));
    }
  };
}

macro_rules! already_borrow_panic {
  ($this: ident) => {
    #[cfg(debug_assertions)]
    {
      let location = $this.value.inner.borrowed_at.get().unwrap();
      panic!("already borrowed at {}", location);
    }
    #[cfg(not(debug_assertions))]
    panic!("already borrowed");
  };
}

impl<'a, W> Deref for StateRef<'a, W> {
  type Target = W;

  #[track_caller]
  fn deref(&self) -> &Self::Target {
    if self.mut_accessed_flag.get().is_none() {
      self.mut_accessed_flag.set(Some(false));
      let b = &self.value.inner.borrow_flag;
      b.set(b.get() + 1);

      match b.get() {
        1 => {
          debug_borrow_location!(self);
        }
        flag if !is_reading(flag) => {
          already_borrow_panic!(self);
        }
        _ => {}
      }
      if !is_reading(b.get()) {}
    }

    // SAFETY: `BorrowFlag` guarantees unique access.
    let ptr = self.value.inner.data.get();
    unsafe { &*ptr }
  }
}

impl<'a, W> DerefMut for StateRef<'a, W> {
  #[track_caller]
  fn deref_mut(&mut self) -> &mut Self::Target {
    let b = &self.value.inner.borrow_flag;
    if log::log_enabled!(log::Level::Debug) {
      let caller = std::panic::Location::caller();
      log::debug!("state modified at {caller}");
    }

    match self.mut_accessed_flag.get() {
      None => {
        debug_borrow_location!(self);
        b.set(b.get() - 1);
        self.mut_accessed_flag.set(Some(true))
      }
      Some(false) => {
        debug_borrow_location!(self);

        // current ref is borrowed value, we release the borrow and mutably
        // borrow the value.
        b.set(b.get() - 2);
        self.mut_accessed_flag.set(Some(true))
      }
      Some(true) => {
        // Already mutably the value, directly return.
      }
    }

    if b.get() != -1 {
      already_borrow_panic!(self);
    }

    // SAFETY: `BorrowFlag` guarantees unique access.
    let ptr = self.value.inner.data.get();
    unsafe { &mut *ptr }
  }
}

impl<'a, W> StateRef<'a, W> {
  /// Fork a silent reference
  pub fn silent(&self) -> StateRef<'a, W> {
    self.release_borrow();
    StateRef::new(self.value.inner_ref(), ModifyScope::DATA)
  }

  /// Forget all modifies record in this reference. So the downstream will no
  /// know the inner value be modified if this reference not be mut accessed
  /// anymore.
  pub fn forget_modifies(&self) {
    let b = &self.value.inner.borrow_flag;
    match self.mut_accessed_flag.get() {
      Some(false) => b.set(b.get() - 1),
      Some(true) => b.set(b.get() + 1),
      None => {}
    }
    self.mut_accessed_flag.set(None);
  }

  #[inline]
  pub fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.value.raw_modifies()
  }

  #[inline]
  pub fn modifies(&self) -> BoxOp<'static, (), Infallible> { self.value.modifies() }

  fn new(value: &'a Stateful<W>, modify_scope: ModifyScope) -> Self {
    Self {
      mut_accessed_flag: Cell::new(None),
      modify_scope,
      value: ModifyGuard::new(value),
    }
  }

  #[inline]
  fn release_borrow(&self) {
    let b = &self.value.inner.borrow_flag;
    match self.mut_accessed_flag.get() {
      Some(false) => b.set(b.get() - 1),
      Some(true) => {
        b.set(b.get() + 1);
        let scope = &self.value.inner.modify_scope;
        let union_scope = scope
          .get()
          .map_or(self.modify_scope, |s| s.union(self.modify_scope));
        scope.set(Some(union_scope));
      }
      None => {}
    }
    self.mut_accessed_flag.set(None);
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.

  #[inline]
  pub fn clone_stateful(&self) -> Stateful<W> { self.value.clone() }
}

impl<W: SingleChild> SingleChild for Stateful<W> {}
impl<W: MultiChild> MultiChild for Stateful<W> {}

impl<W: Render + 'static> Render for Stateful<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.state_ref().perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.state_ref().only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.state_ref().paint(ctx) }

  #[inline]
  fn can_overflow(&self) -> bool { self.state_ref().can_overflow() }

  #[inline]
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    self.state_ref().hit_test(ctx, pos)
  }

  #[inline]
  fn get_transform(&self) -> Option<Transform> { self.state_ref().get_transform() }
}

impl<'a, W> Drop for StateRef<'a, W> {
  fn drop(&mut self) { self.release_borrow(); }
}

impl<W: Query + 'static> Query for Stateful<W> {
  impl_proxy_query!(self.modify_notifier, self.state_ref());
}

impl Query for StateChangeNotifier {
  impl_query_self_only!();
}

impl StateChangeNotifier {
  pub(crate) fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> { self.0.clone() }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    // Simulate `MockBox` widget need modify its size in event callback. Can use the
    // `cell_ref` in closure.
    let stateful = Stateful::new(MockBox { size: Size::zero() });
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

    let sized_box = Stateful::new(MockBox { size: Size::new(100., 100.) });
    sized_box
      .modifies()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    let c_box = sized_box.clone();
    sized_box.modifies().subscribe(move |_| {
      *c_changed_size.borrow_mut() = c_box.state_ref().size;
    });

    let state = sized_box.clone();
    let mut wnd = Window::default_mock(sized_box.into_widget(), None);
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 0);
    assert!(!wnd.widget_tree.is_dirty());
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));

    {
      state.state_ref().size = Size::new(1., 1.);
    }
    assert!(wnd.widget_tree.is_dirty());
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    let mut wnd = Window::default_mock(widget! { MockBox { size: Size::new(100., 100.) } }, None);
    wnd.draw_frame();
    let tree = &wnd.widget_tree;
    assert_eq!(tree.root().descendants(&tree.arena).count(), 1);
  }

  #[test]
  fn change_notify() {
    let notified = Rc::new(RefCell::new(vec![]));
    let c_notified = notified.clone();
    let w = Stateful::new(MockBox { size: Size::zero() });
    w.raw_modifies()
      .subscribe(move |b| c_notified.borrow_mut().push(b));

    {
      let _ = &mut w.state_ref().size;
    }
    assert_eq!(notified.borrow().deref(), &[ModifyScope::BOTH]);

    {
      let _ = &mut w.silent_ref().size;
    }
    assert_eq!(
      notified.borrow().deref(),
      &[ModifyScope::BOTH, ModifyScope::DATA]
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
      &[ModifyScope::BOTH, ModifyScope::DATA]
    );
  }
}
