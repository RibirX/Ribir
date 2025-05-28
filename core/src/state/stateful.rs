use std::{cell::Cell, convert::Infallible};

use ribir_algo::Sc;
use rxrust::ops::box_it::CloneableBoxOp;

use crate::prelude::*;

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  data: Sc<StateCell<W>>,
  info: Sc<WriterInfo>,
}

pub struct Reader<W>(pub(crate) Sc<StateCell<W>>);

/// The notifier is a `RxRust` stream that emit notification when the state
/// changed.
#[derive(Default, Clone)]
pub struct Notifier(Subject<'static, ModifyInfo, Infallible>);

bitflags! {
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub struct ModifyScope: u8 {
    /// state change only effect the data, transparent to ribir framework.
    const DATA  = 1 << 0;
    /// state change only effect to framework, transparent to widget data.
    const FRAMEWORK = 1 << 1;
    /// state change effect both widget data and framework.
    const BOTH = Self::DATA.bits() | Self::FRAMEWORK.bits();
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyInfo {
  pub(crate) scope: ModifyScope,
  pub(crate) partial: Option<PartialPath>,
}

impl ModifyInfo {
  pub fn new(scope: ModifyScope, partial: Option<PartialPath>) -> Self { Self { scope, partial } }

  pub fn contains(&self, scope: ModifyScope) -> bool { self.scope.contains(scope) }

  pub fn scope(&self) -> ModifyScope { self.scope }

  pub fn partial_path(&self) -> Option<&PartialPath> { self.partial.as_ref() }
}

impl Notifier {
  pub(crate) fn unsubscribe(&mut self) { self.0.clone().unsubscribe(); }
}

pub(crate) struct WriterInfo {
  pub(crate) notifier: Notifier,
  /// The counter of the writer may be modified the data.
  pub(crate) writer_count: Cell<usize>,
  /// The batched modifies of the `State` which will be notified.
  pub(crate) batched_modifies: Cell<ModifyScope>,
}

impl<W: 'static> StateReader for Stateful<W> {
  type Value = W;
  type Reader = Reader<W>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.data.read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { Reader(self.data.clone()) }

  fn try_into_value(self) -> Result<W, Self> {
    if self.data.ref_count() == 1 {
      let data = self.data.clone();
      drop(self);
      // SAFETY: `self.data.ref_count() == 1` guarantees unique access.
      let data = unsafe { Sc::try_unwrap(data).unwrap_unchecked() };
      Ok(data.into_inner())
    } else {
      Err(self)
    }
  }
}

impl<W: 'static> StateWatcher for Stateful<W> {
  type Watcher = Watcher<Self::Reader>;

  fn into_reader(self) -> Result<Self::Reader, Self> {
    if self.info.writer_count.get() == 1 { Ok(self.clone_reader()) } else { Err(self) }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.info.notifier.raw_modifies()
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<W: 'static> StateWriter for Stateful<W> {
  #[inline]
  fn write(&self) -> WriteRef<W> { self.write_ref(ModifyScope::BOTH) }

  #[inline]
  fn silent(&self) -> WriteRef<W> { self.write_ref(ModifyScope::DATA) }

  #[inline]
  fn shallow(&self) -> WriteRef<W> { self.write_ref(ModifyScope::FRAMEWORK) }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  #[inline]
  fn clone_writer(&self) -> Self { self.clone() }
}

impl<W: 'static> StateReader for Reader<W> {
  type Value = W;
  type Reader = Self;

  #[inline]
  fn read(&self) -> ReadRef<W> { self.0.read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self { Reader(self.0.clone()) }

  fn try_into_value(self) -> Result<Self::Value, Self> {
    if self.0.ref_count() == 1 {
      // SAFETY: `self.0.ref_count() == 1` guarantees unique access.
      let data = unsafe { Sc::try_unwrap(self.0).unwrap_unchecked() };
      Ok(data.into_inner())
    } else {
      Err(self)
    }
  }
}

impl<W> Drop for Stateful<W> {
  fn drop(&mut self) { self.info.dec_writer(); }
}

impl Drop for WriterInfo {
  fn drop(&mut self) {
    if self.writer_count.get() == 0 {
      let mut notifier = self.notifier.clone();
      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      let _ = AppCtx::spawn_local(async move { notifier.unsubscribe() });
    }
  }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Self { data: Sc::new(StateCell::new(data)), info: Sc::new(WriterInfo::new()) }
  }

  /// Determines if two `Stateful` instances point to the same underlying data.
  ///
  /// Performs pointer equality checks on both:
  /// - The state data container
  /// - The associated metadata
  #[inline]
  pub fn ptr_eq(this: &Self, other: &Self) -> bool {
    Sc::ptr_eq(&this.data, &other.data) && Sc::ptr_eq(&this.info, &other.info)
  }

  pub fn from_pipe(p: impl Pipe<Value = W>) -> (Self, BoxSubscription<'static>)
  where
    Self: 'static,
  {
    let (v, p) = p.unzip(ModifyScope::DATA, None);
    let s = Stateful::new(v);
    let s2 = s.clone_writer();
    let u = p.subscribe(move |(_, v)| *s2.write() = v);
    (s, u)
  }

  fn write_ref(&self, scope: ModifyScope) -> WriteRef<'_, W> {
    let value = self.data.write();
    WriteRef { value, modified: false, modify_scope: scope, info: &self.info, partial: None }
  }

  fn clone(&self) -> Self {
    self.info.inc_writer();
    Self { data: self.data.clone(), info: self.info.clone() }
  }
}

impl WriterInfo {
  pub(crate) fn new() -> Self {
    WriterInfo {
      batched_modifies: <_>::default(),
      writer_count: Cell::new(1),
      notifier: <_>::default(),
    }
  }

  pub(crate) fn inc_writer(&self) { self.writer_count.set(self.writer_count.get() + 1); }

  pub(crate) fn dec_writer(&self) { self.writer_count.set(self.writer_count.get() - 1); }
}

impl Notifier {
  pub(crate) fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.0.clone().box_it()
  }

  pub(crate) fn next(&self, scope: ModifyInfo) { self.0.clone().next(scope) }
}

impl<W: std::fmt::Debug> std::fmt::Debug for Stateful<W> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Stateful")
      .field(&*self.data.read())
      .finish()
  }
}

impl<W: Default> Default for Stateful<W> {
  fn default() -> Self { Self::new(W::default()) }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use super::*;
  use crate::test_helper::*;

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn smoke() {
    crate::reset_test_env!();

    // Simulate `MockBox` widget need modify its size in event callback. Can use the
    // `cell_ref` in closure.
    let stateful = Stateful::new(MockBox { size: Size::zero() });
    {
      stateful.write().size = Size::new(100., 100.)
    }
    assert_eq!(stateful.read().size, Size::new(100., 100.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn unsubscribe_when_not_writer() {
    crate::reset_test_env!();
    struct Guard {
      drop_cnt: Rc<RefCell<i32>>,
    }
    impl Guard {
      fn _use(&self) {}
    }
    impl Drop for Guard {
      fn drop(&mut self) { *self.drop_cnt.borrow_mut() += 1; }
    }

    fn drop_writer_subscribe<W: StateWriter>(w: W, drop_cnt: Rc<RefCell<i32>>) {
      let guard = Guard { drop_cnt: drop_cnt.clone() };
      let r = w.clone_reader();
      w.modifies().subscribe(move |_| {
        guard._use();
        r.clone_reader();
      });
    }

    let drop_cnt = Rc::new(RefCell::new(0));
    {
      drop_writer_subscribe(Stateful::new(()), drop_cnt.clone());
    };
    AppCtx::run_until_stalled();
    assert_eq!(*drop_cnt.borrow(), 1);

    {
      drop_writer_subscribe(
        #[allow(clippy::redundant_closure)]
        Stateful::new(()).part_writer(None, |v| PartMut::new(v)),
        drop_cnt.clone(),
      );
    };
    AppCtx::run_until_stalled();
    assert_eq!(*drop_cnt.borrow(), 2);

    {
      drop_writer_subscribe(
        #[allow(clippy::redundant_closure)]
        Stateful::new(()).part_writer(Some(""), |v| PartMut::new(v)),
        drop_cnt.clone(),
      );
    };
    AppCtx::run_until_stalled();
    assert_eq!(*drop_cnt.borrow(), 3);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    let mut wnd = TestWindow::new(fn_widget! { sized_box.clone_writer() });
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 0);
    assert!(!wnd.tree().is_dirty());
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));

    {
      state.write().size = Size::new(1., 1.);
    }
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert!(wnd.tree().is_dirty());
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 1);
    assert_eq!(&*changed_size.borrow(), &Size::new(1., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn fix_pin_widget_node() {
    crate::reset_test_env!();

    let mut wnd = TestWindow::new(fn_widget!(MockBox { size: Size::new(100., 100.) }));
    wnd.draw_frame();
    let tree = wnd.tree();
    assert_eq!(tree.content_root().descendants(tree).count(), 1);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn change_notify() {
    crate::reset_test_env!();

    let notified = Sc::new(RefCell::new(vec![]));
    let c_notified = notified.clone();
    let w = Stateful::new(MockBox { size: Size::zero() });
    w.raw_modifies()
      .subscribe(move |b| c_notified.borrow_mut().push(b));

    {
      let _ = &mut w.write().size;
    }
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(
      &notified
        .borrow()
        .iter()
        .map(|s| s.scope())
        .collect::<Vec<_>>(),
      &[ModifyScope::BOTH]
    );

    {
      let _ = &mut w.silent().size;
    }

    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(
      &notified
        .borrow()
        .iter()
        .map(|s| s.scope())
        .collect::<Vec<_>>(),
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
      &notified
        .borrow()
        .iter()
        .map(|s| s.scope())
        .collect::<Vec<_>>(),
      &[ModifyScope::BOTH, ModifyScope::DATA]
    );
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn render_only_hold_data() {
    crate::reset_test_env!();

    // data + 1, info + 1
    let v = Stateful::new(1);
    // data + 1
    let _r = v.clone_reader();

    AppCtx::run_until_stalled();
    assert_eq!(v.data.ref_count(), 2);
    assert_eq!(v.info.ref_count(), 1);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_only_hold_data() {
    crate::reset_test_env!();

    // data + 1, info + 1
    let v = Stateful::new(1);
    // data +1
    let (_, stream) = pipe!(*$v).unzip(ModifyScope::all(), None);
    let _ = stream.subscribe(|(_, v)| println!("{v}"));

    AppCtx::run_until_stalled();
    assert_eq!(v.data.ref_count(), 2);
    assert_eq!(v.info.ref_count(), 1);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn watch_only_hold_data() {
    crate::reset_test_env!();

    // data + 1, info + 1
    let v = Stateful::new(1);
    // data + 1
    let _ = watch!(*$v).subscribe(|v| println!("{v}"));

    AppCtx::run_until_stalled();
    assert_eq!(v.data.ref_count(), 2);
    assert_eq!(v.info.ref_count(), 1);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn render_in_downstream_no_circle() {}

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn writer_in_downstream_unsubscribe() {
    crate::reset_test_env!();

    let v = Stateful::new(1);
    let data = v.data.clone();
    let notifier = v.info.notifier.0.clone();
    {
      let info = v.info.clone();
      let u = watch!(*$v).subscribe(move |_| *v.write() = 2);
      u.unsubscribe();
      AppCtx::run_until_stalled();
      assert_eq!(info.ref_count(), 1);
    }

    AppCtx::run_until_stalled();

    assert_eq!(*data.read(), 2);
    assert!(notifier.is_closed());
  }
}
