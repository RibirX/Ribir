use std::{cell::Cell, convert::Infallible};

use ribir_algo::Sc;
use rxrust::ops::box_it::CloneableBoxOp;
use smallvec::{SmallVec, smallvec};

use crate::prelude::*;

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  data: Sc<StateCell<W>>,
  info: Sc<WriterInfo>,
  include_partial: bool,
}

pub struct Reader<W>(pub(crate) Sc<StateCell<W>>);

/// The notifier is a `rxRust` stream that emit notification when the state
/// changed.
#[derive(Default, Clone)]
pub struct Notifier(Subject<'static, ModifyInfo, Infallible>);

bitflags! {
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub struct ModifyEffect: u8 {
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
  pub(crate) effect: ModifyEffect,
  pub(crate) path: SmallVec<[PartialId; 1]>,
}

impl ModifyInfo {
  /// Check if the effect is contained in this modify info.
  pub fn contains(&self, effect: ModifyEffect) -> bool { self.effect.contains(effect) }
}

impl Notifier {
  pub(crate) fn unsubscribe(&mut self) { self.0.clone().unsubscribe(); }
}

pub(crate) struct WriterInfo {
  pub(crate) notifier: Notifier,
  /// The counter of the writer may be modified the data.
  pub(crate) writer_count: Cell<usize>,
  /// The batched modifies of the `State` which will be notified.
  pub(crate) batched_modifies: Cell<ModifyEffect>,
}

impl<W: 'static> StateReader for Stateful<W> {
  type Value = W;
  type Reader = Reader<W>;

  #[inline]
  fn read(&self) -> ReadRef<'_, Self::Value> { self.data.read() }

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

  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    let include_partial = self.include_partial;
    let mut modifies = self.info.notifier.raw_modifies();
    if !include_partial {
      modifies = modifies
        .filter(move |info| info.path.iter().all(|id| id == &PartialId::ANY))
        .box_it();
    }
    modifies
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<W: 'static> StateWriter for Stateful<W> {
  #[inline]
  fn write(&self) -> WriteRef<'_, W> { self.write_ref(ModifyEffect::BOTH) }

  #[inline]
  fn silent(&self) -> WriteRef<'_, W> { self.write_ref(ModifyEffect::DATA) }

  #[inline]
  fn shallow(&self) -> WriteRef<'_, W> { self.write_ref(ModifyEffect::FRAMEWORK) }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  #[inline]
  fn clone_writer(&self) -> Self { self.clone() }

  fn include_partial_writers(&mut self, include: bool) { self.include_partial = include; }

  #[inline]
  fn scope_path(&self) -> SmallVec<[PartialId; 1]> { smallvec![] }
}

impl<W: 'static> StateReader for Reader<W> {
  type Value = W;
  type Reader = Self;

  #[inline]
  fn read(&self) -> ReadRef<'_, W> { self.0.read() }

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
      AppCtx::spawn_local(async move { notifier.unsubscribe() });
    }
  }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Self {
      data: Sc::new(StateCell::new(data)),
      info: Sc::new(WriterInfo::new()),
      include_partial: false,
    }
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

  pub fn from_pipe(p: Pipe<W>) -> (Self, BoxSubscription<'static>)
  where
    W: Default,
    Self: 'static,
  {
    let s = Stateful::new(W::default());
    let s2 = s.clone_writer();
    let u = p
      .into_observable()
      .subscribe(move |v| *s2.write() = v);
    (s, u)
  }

  fn write_ref(&self, effect: ModifyEffect) -> WriteRef<'_, W> {
    let path = smallvec![];
    WriteRef::new(self.data.write(), &self.info, path, effect)
  }

  fn clone(&self) -> Self {
    self.info.inc_writer();
    Self { data: self.data.clone(), info: self.info.clone(), include_partial: self.include_partial }
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

impl Default for ModifyInfo {
  fn default() -> Self {
    ModifyInfo { path: smallvec![PartialId::any()], effect: ModifyEffect::BOTH }
  }
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
        Stateful::new(()).part_writer(PartialId::any(), |v| PartMut::new(v)),
        drop_cnt.clone(),
      );
    };
    AppCtx::run_until_stalled();
    assert_eq!(*drop_cnt.borrow(), 2);

    {
      drop_writer_subscribe(
        #[allow(clippy::redundant_closure)]
        Stateful::new(()).part_writer("".into(), |v| PartMut::new(v)),
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
    let wnd = TestWindow::from_widget(fn_widget! { sized_box.clone_writer() });
    wnd.draw_frame();
    assert_eq!(*notified_count.borrow(), 0);
    assert!(!wnd.tree().is_dirty());
    assert_eq!(&*changed_size.borrow(), &Size::new(0., 0.));

    {
      state.write().size = Size::new(1., 1.);
    }
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

    let wnd = TestWindow::from_widget(fn_widget!(MockBox { size: Size::new(100., 100.) }));
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
    AppCtx::run_until_stalled();

    assert_eq!(
      &notified
        .borrow()
        .iter()
        .map(|s| s.effect)
        .collect::<Vec<_>>(),
      &[ModifyEffect::BOTH]
    );

    {
      let _ = &mut w.silent().size;
    }

    AppCtx::run_until_stalled();
    assert_eq!(
      &notified
        .borrow()
        .iter()
        .map(|s| s.effect)
        .collect::<Vec<_>>(),
      &[ModifyEffect::BOTH, ModifyEffect::DATA]
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

    AppCtx::run_until_stalled();
    assert_eq!(
      &notified
        .borrow()
        .iter()
        .map(|s| s.effect)
        .collect::<Vec<_>>(),
      &[ModifyEffect::BOTH, ModifyEffect::DATA]
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
    let _ = pipe!(*$read(v))
      .into_observable()
      .subscribe(|v| println!("{v}"));

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
    let _ = watch!(*$read(v)).subscribe(|v| println!("{v}"));

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
      let u = watch!(*$read(v)).subscribe(move |_| *v.write() = 2);
      u.unsubscribe();
      AppCtx::run_until_stalled();
      assert_eq!(info.ref_count(), 1);
    }

    AppCtx::run_until_stalled();

    assert_eq!(*data.read(), 2);
    assert!(notifier.is_closed());
  }
}
