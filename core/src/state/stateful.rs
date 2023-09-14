use crate::{impl_proxy_query, impl_proxy_render, impl_query_self_only, prelude::*};
use ribir_algo::Sc;
use rxrust::{ops::box_it::BoxOp, prelude::*};
use std::{
  cell::{Cell, Ref, RefCell, RefMut},
  convert::Infallible,
  ops::{Deref, DerefMut},
};

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  pub(crate) inner: Sc<StateData<W>>,
  pub(crate) notifier: Notifier,
}

pub struct Reader<W>(Stateful<W>);

pub struct Writer<W>(Stateful<W>);

pub struct ReadRef<'a, W>(Ref<'a, W>);

pub struct WriteRef<'a, W> {
  modified: bool,
  modify_scope: ModifyScope,
  value: RefMut<'a, W>,
  batched_modify: &'a Sc<Cell<ModifyScope>>,
  notifier: Option<&'a Notifier>,
}

/// The notifier is a `RxRust` stream that emit notification when the state
/// changed.
#[derive(Default, Clone)]
pub struct Notifier(Subject<'static, ModifyScope, Infallible>);

bitflags! {
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub struct ModifyScope: u8 {
    /// state change only effect the data, transparent to ribir framework.
    const DATA  = 0x01;
    /// state change only effect to framework, transparent to widget data.
    const FRAMEWORK = 0x010;
    /// state change effect both widget data and framework.
    const BOTH = Self::DATA.bits() | Self::FRAMEWORK.bits();
  }
}

impl<W> StateReader for Stateful<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Reader<W>;
  type Ref<'a> = ReadRef<'a, W> where Self: 'a,;

  #[inline]
  fn read(&self) -> ReadRef<'_, W> { self.inner.read() }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { Reader::from_stateful(self) }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.notifier.modifies() }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.notifier.raw_modifies()
  }
}

impl<W> StateWriter for Stateful<W> {
  type Writer = Writer<W>;
  type OriginWriter = Self;
  type RefWrite<'a> = WriteRef<'a, W> where Self: 'a;

  fn write(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::BOTH) }

  fn silent(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::DATA) }

  fn shallow(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::FRAMEWORK) }

  fn clone_writer(&self) -> Self::Writer { Writer::from_stateful(self) }

  fn origin_writer(&self) -> &Self::OriginWriter { self }
}

impl<W> StateReader for Reader<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Self;
  type Ref<'a> = ReadRef<'a, W> where W:'a;

  #[inline]
  fn read(&'_ self) -> Self::Ref<'_> { self.0.read() }

  #[inline]
  fn clone_reader(&self) -> Self { self.0.clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.0.modifies() }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> { self.0.raw_modifies() }
}

impl<W> StateReader for Writer<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Reader<W>;
  type Ref<'a> = ReadRef<'a, W> where W:'a;

  #[inline]
  fn read(&'_ self) -> Self::Ref<'_> { self.0.read() }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.0.clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.0.modifies() }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> { self.0.raw_modifies() }
}

impl<W> StateWriter for Writer<W> {
  type Writer = Self;
  type OriginWriter = Self;
  type RefWrite<'a> = WriteRef<'a, W> where W:'a;

  #[inline]
  fn write(&'_ self) -> Self::RefWrite<'_> { self.0.write() }

  #[inline]
  fn silent(&'_ self) -> Self::RefWrite<'_> { self.0.silent() }

  #[inline]
  fn shallow(&'_ self) -> Self::RefWrite<'_> { self.0.shallow() }

  #[inline]
  fn clone_writer(&self) -> Self { self.0.clone_writer() }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { self }
}

impl<W> Drop for Reader<W> {
  fn drop(&mut self) {
    // The `Stateful` is a writer but used as a reader in `Reader` that not
    // increment the writer count. So we increment the writer count before drop the
    // `Stateful` to keep its count correct.
    self.0.inc_writer();
  }
}

impl<W> Drop for Stateful<W> {
  fn drop(&mut self) {
    self.dec_writer();
    // can cancel the notifier, because no one will modify the data.
    if self.writer_count() == 0 {
      let notifier = self.notifier.clone();
      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      AppCtx::spawn_local(async move {
        notifier.0.unsubscribe();
      })
      .unwrap();
    }

    // Declare object may add task to disconnect to upstream, trigger that task if
    // this is the last reference. We not hold that task in `Stateful` to avoid
    // cycle reference.
    if self.inner.ref_count() == 1 {
      AppCtx::trigger_task(self.heap_ptr() as *const ());
    }
  }
}

impl<W> Reader<W> {
  fn from_stateful(stateful: &Stateful<W>) -> Self {
    Reader(Stateful {
      inner: stateful.inner.clone(),
      notifier: stateful.notifier.clone(),
    })
  }
}

impl<W> Writer<W> {
  #[inline]
  pub fn into_inner(self) -> Stateful<W> { self.0 }

  fn from_stateful(stateful: &Stateful<W>) -> Self {
    stateful.inc_writer();
    Writer(Stateful {
      inner: stateful.inner.clone(),
      notifier: stateful.notifier.clone(),
    })
  }
}

pub(crate) struct StateData<W> {
  data: RefCell<W>,
  /// The batched modifies of the `State` which will be notified.
  batch_modified: Sc<Cell<ModifyScope>>,
  /// The counter of the writer may be modified the data.
  writer_count: Cell<usize>,
}

#[repr(transparent)]
pub(crate) struct RenderFul<R>(pub(crate) Stateful<R>);

impl_proxy_query!(paths [0], RenderFul<R>, <R>, where R: Render + 'static);
impl_proxy_render!(proxy 0.read(), RenderFul<R>, <R>, where R: Render + 'static);
impl_proxy_query!(paths[0.read()], RenderFul<Box<dyn Render>>);
impl_proxy_render!(proxy 0.read(), RenderFul<Box<dyn Render>>);

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Stateful {
      inner: Sc::new(StateData::new(data)),
      notifier: <_>::default(),
    }
  }

  /// Clone the stateful widget of which the reference point to. Require mutable
  /// reference because we try to early release inner borrow when clone occur.
  #[inline]
  pub fn clone_stateful(&self) -> Stateful<W> { self.clone_writer().0 }

  /// just for compatibility with `Stateful` in old syntax.
  #[inline]
  pub fn state_ref(&self) -> WriteRef<W> { self.write() }

  /// Run the `task` when the inner state data will drop.
  #[inline]
  pub fn on_state_drop(&self, task: impl FnOnce() + 'static) {
    AppCtx::add_trigger_task(self.heap_ptr() as *const _, Box::new(task))
  }

  // unsubscribe the `subscription` when the inner state data will drop.
  #[inline]
  pub fn unsubscribe_on_drop(&self, subscription: impl Subscription + 'static) {
    self.on_state_drop(move || subscription.unsubscribe())
  }

  /// return the heap pointer of the data.
  #[inline]
  fn heap_ptr(&self) -> *const W { self.inner.data.as_ptr() }

  pub(crate) fn from_state_data(data: StateData<W>) -> Self {
    Self {
      inner: Sc::new(data),
      notifier: <_>::default(),
    }
  }

  pub(crate) fn try_into_inner(self) -> Result<W, Self> {
    if Sc::ref_count(&self.inner) == 1 {
      let inner = self.inner.clone();
      drop(self);
      // SAFETY: `Rc::strong_count(&self.inner) == 1` guarantees unique access.
      let inner = unsafe { Sc::try_unwrap(inner).unwrap_unchecked() };
      Ok(inner.data.into_inner())
    } else {
      Err(self)
    }
  }

  fn write_ref(&self, scope: ModifyScope) -> WriteRef<'_, W> {
    let value = self.inner.data.borrow_mut();
    let batched_modify = &self.inner.batch_modified;
    let modifier = &self.notifier;
    WriteRef::new(value, scope, batched_modify, Some(modifier))
  }

  fn writer_count(&self) -> usize { self.inner.writer_count.get() }
  fn inc_writer(&self) { self.inner.writer_count.set(self.writer_count() + 1); }
  fn dec_writer(&self) { self.inner.writer_count.set(self.writer_count() - 1); }
}

impl<W> StateData<W> {
  /// Assert the state data is not used by any reader and writer.
  #[inline]
  #[track_caller]
  pub(crate) fn assert_is_not_used(&self) { self.data.borrow_mut(); }

  #[inline]
  pub(crate) fn new(data: W) -> Self {
    Self {
      // the `StateData` in `Stateful` or `State` is a writer
      writer_count: Cell::new(1),
      data: RefCell::new(data),
      batch_modified: <_>::default(),
    }
  }

  pub(crate) fn into_inner(self) -> W { self.data.into_inner() }

  pub(crate) fn read(&self) -> ReadRef<W> { ReadRef(self.data.borrow()) }
}

impl<'a, W> Deref for ReadRef<'a, W> {
  type Target = W;

  #[track_caller]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a, W> Deref for WriteRef<'a, W> {
  type Target = W;
  #[track_caller]
  fn deref(&self) -> &Self::Target { &self.value }
}

impl<'a, W> DerefMut for WriteRef<'a, W> {
  #[track_caller]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.modified = true;
    &mut self.value
  }
}

impl<'a, W> RefWrite for WriteRef<'a, W> {
  #[inline]
  fn forget_modifies(&mut self) -> bool { std::mem::replace(&mut self.modified, false) }
}

impl<'a, W> WriteRef<'a, W> {
  pub(crate) fn new(
    value: RefMut<'a, W>,
    scope: ModifyScope,
    batch_scope: &'a Sc<Cell<ModifyScope>>,
    modifier: Option<&'a Notifier>,
  ) -> Self {
    Self {
      modified: false,
      modify_scope: scope,
      value,
      batched_modify: batch_scope,
      notifier: modifier,
    }
  }
}

impl<W: SingleChild> SingleChild for Stateful<W> {}
impl<W: MultiChild> MultiChild for Stateful<W> {}

impl_proxy_query!(
  paths [notifier, read()],
  Stateful<R>, <R>, where R: Query + 'static
);
impl_query_self_only!(Notifier);
impl_proxy_query!(paths [0], Reader<T>, <T>, where T: Query + 'static);
impl_proxy_query!(paths [0], Writer<T>, <T>, where T: Query + 'static);

impl<'a, W> Drop for WriteRef<'a, W> {
  fn drop(&mut self) {
    if !self.modified {
      return;
    }

    let scope = self.modify_scope;
    let batch_scope = self.batched_modify.get();
    if batch_scope.is_empty() && !scope.is_empty() {
      self.batched_modify.set(scope);
      if let Some(m) = self.notifier.as_mut() {
        let mut subject = m.raw_modifies();
        let share_scope = self.batched_modify.clone();
        AppCtx::spawn_local(async move {
          let scope = share_scope.replace(ModifyScope::empty());
          subject.next(scope);
        })
        .unwrap();
      }
    } else {
      self.batched_modify.set(batch_scope | scope);
    }
  }
}

impl Notifier {
  pub fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self
      .raw_modifies()
      .filter(|s| s.contains(ModifyScope::DATA))
      .box_it()
  }

  pub(crate) fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> { self.0.clone() }
}

impl<W: std::fmt::Debug> std::fmt::Debug for Stateful<W> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Stateful").field(&*self.read()).finish()
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;
  use crate::{test_helper::*, timer::Timer};

  #[test]
  fn smoke() {
    crate::reset_test_env!();

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
    crate::reset_test_env!();

    use std::{cell::RefCell, rc::Rc};
    let notified_count = Rc::new(RefCell::new(0));
    let cnc = notified_count.clone();

    let sized_box = Stateful::new(MockBox { size: Size::new(100., 100.) });
    sized_box
      .modifies()
      .subscribe(move |_| *cnc.borrow_mut() += 1);

    let changed_size = Rc::new(RefCell::new(Size::zero()));
    let c_changed_size = changed_size.clone();
    let c_box = sized_box.clone_writer();
    sized_box.modifies().subscribe(move |_| {
      *c_changed_size.borrow_mut() = c_box.write().size;
    });

    let state = sized_box.clone_writer();
    let mut wnd = TestWindow::new(sized_box);
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 0);
    assert!(!wnd.widget_tree.borrow().is_dirty());
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));

    {
      state.write().size = Size::new(1., 1.);
    }
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert!(wnd.widget_tree.borrow().is_dirty());
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[test]
  fn fix_pin_widget_node() {
    crate::reset_test_env!();

    let mut wnd = TestWindow::new(MockBox { size: Size::new(100., 100.) });
    wnd.draw_frame();
    let tree = wnd.widget_tree.borrow();
    assert_eq!(tree.root().descendants(&tree.arena).count(), 1);
  }

  #[test]
  fn change_notify() {
    crate::reset_test_env!();

    let notified = Sc::new(RefCell::new(vec![]));
    let c_notified = notified.clone();
    let w = Stateful::new(MockBox { size: Size::zero() });
    w.raw_modifies()
      .subscribe(move |b| c_notified.borrow_mut().push(b));

    {
      let _ = &mut w.state_ref().size;
    }
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(notified.borrow().deref(), &[ModifyScope::BOTH]);

    {
      let _ = &mut w.silent().size;
    }

    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(
      notified.borrow().deref(),
      &[ModifyScope::BOTH, ModifyScope::DATA]
    );

    {
      let _ = &mut w.write();
    }
    {
      let _ = &mut w.write();
    }
    {
      let _ = &mut w.silent();
    }
    {
      let _ = &mut w.silent();
    }

    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(
      notified.borrow().deref(),
      &[ModifyScope::BOTH, ModifyScope::DATA]
    );
  }
}
