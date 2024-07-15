use std::{cell::Cell, convert::Infallible};

use ribir_algo::Sc;
use rxrust::{ops::box_it::CloneableBoxOp, prelude::*};

use super::state_cell::StateCell;
use crate::prelude::*;

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  data: Sc<StateCell<W>>,
  info: Sc<WriterInfo>,
}

pub struct Reader<W>(Sc<StateCell<W>>);

pub struct Writer<W>(Stateful<W>);

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
  type OriginReader = Self;
  type Reader = Reader<W>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.data.read() }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { Reader(self.data.clone()) }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

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
  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.info.notifier.raw_modifies()
  }
}

impl<W: 'static> StateWriter for Stateful<W> {
  type Writer = Writer<W>;
  type OriginWriter = Self;

  #[inline]
  fn write(&self) -> WriteRef<W> { self.write_ref(ModifyScope::BOTH) }

  #[inline]
  fn silent(&self) -> WriteRef<W> { self.write_ref(ModifyScope::DATA) }

  #[inline]
  fn shallow(&self) -> WriteRef<W> { self.write_ref(ModifyScope::FRAMEWORK) }

  #[inline]
  fn clone_writer(&self) -> Self::Writer { Writer(self.clone()) }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { self }
}

impl<W: 'static> StateReader for Reader<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Self;

  #[inline]
  fn read(&self) -> ReadRef<W> { self.0.read() }

  #[inline]
  fn clone_reader(&self) -> Self { Reader(self.0.clone()) }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

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

impl<W: 'static> StateReader for Writer<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Reader<W>;

  #[inline]
  fn read(&'_ self) -> ReadRef<W> { self.0.read() }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.0.clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self> { self.0.try_into_value().map_err(Writer) }
}

impl<V: 'static> StateWatcher for Writer<V> {
  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.0.raw_modifies()
  }
}

impl<V: 'static> StateWriter for Writer<V> {
  type Writer = Self;
  type OriginWriter = Self;

  #[inline]
  fn write(&self) -> WriteRef<V> { self.0.write() }

  #[inline]
  fn silent(&self) -> WriteRef<V> { self.0.silent() }

  #[inline]
  fn shallow(&self) -> WriteRef<V> { self.0.shallow() }

  #[inline]
  fn clone_writer(&self) -> Self { self.0.clone_writer() }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { self }
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

impl<W> Writer<W> {
  #[inline]
  pub fn into_inner(self) -> Stateful<W> { self.0 }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Self { data: Sc::new(StateCell::new(data)), info: Sc::new(WriterInfo::new()) }
  }

  fn write_ref(&self, scope: ModifyScope) -> WriteRef<'_, W> {
    let value = self.data.write();
    WriteRef { value, modified: false, modify_scope: scope, info: &self.info }
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

impl<W: Render + 'static> IntoWidgetStrict<'static, RENDER> for Stateful<W> {
  fn into_widget_strict(self) -> Widget<'static> {
    match self.try_into_value() {
      Ok(r) => r.into_widget(),
      Err(s) => {
        let f = move |ctx: &BuildCtx| {
          let w = s.data.clone().into_widget().build(ctx);
          w.dirty_subscribe(s.raw_modifies(), ctx);
          w
        };
        InnerWidget::LazyBuild(Box::new(f)).into()
      }
    }
  }
}

impl<W: Compose + 'static> IntoWidgetStrict<'static, COMPOSE> for Stateful<W> {
  fn into_widget_strict(self) -> Widget<'static> { Compose::compose(self) }
}

impl<W: Render> IntoWidgetStrict<'static, RENDER> for Reader<W> {
  fn into_widget_strict(self) -> Widget<'static> {
    match Sc::try_unwrap(self.0) {
      Ok(r) => r.into_inner().into_widget(),
      Err(s) => s.into_widget(),
    }
  }
}

impl<'w, W, const M: usize> IntoWidgetStrict<'w, M> for Writer<W>
where
  Stateful<W>: IntoWidget<'w, M>,
{
  fn into_widget_strict(self) -> Widget<'w> { self.0.into_widget() }
}

impl Notifier {
  pub(crate) fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.0.clone().box_it()
  }

  pub(crate) fn next(&self, scope: ModifyScope) { self.0.clone().next(scope) }
}

impl<W: std::fmt::Debug> std::fmt::Debug for Stateful<W> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Stateful")
      .field(&*self.data.read())
      .finish()
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
        Stateful::new(()).map_writer(|v| PartData::from_ref_mut(v)),
        drop_cnt.clone(),
      );
    };
    AppCtx::run_until_stalled();
    assert_eq!(*drop_cnt.borrow(), 2);

    {
      drop_writer_subscribe(
        #[allow(clippy::redundant_closure)]
        Stateful::new(()).split_writer(|v| PartData::from_ref_mut(v)),
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
    let mut wnd = TestWindow::new(fn_widget! {sized_box});
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

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn fix_pin_widget_node() {
    crate::reset_test_env!();

    let mut wnd = TestWindow::new(fn_widget!(MockBox { size: Size::new(100., 100.) }));
    wnd.draw_frame();
    let tree = wnd.widget_tree.borrow();
    assert_eq!(tree.content_root().descendants(&tree).count(), 1);
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

    assert_eq!(&*notified.borrow(), &[ModifyScope::BOTH]);

    {
      let _ = &mut w.silent().size;
    }

    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(&*notified.borrow(), &[ModifyScope::BOTH, ModifyScope::DATA]);

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
    assert_eq!(&*notified.borrow(), &[ModifyScope::BOTH, ModifyScope::DATA]);
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
      assert_eq!(info.ref_count(), 1);
    }

    AppCtx::run_until_stalled();

    assert_eq!(data.ref_count(), 1);
    assert!(notifier.is_closed());
  }
}
