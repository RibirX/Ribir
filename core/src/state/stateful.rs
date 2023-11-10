use crate::{prelude::*, render_helper::RenderProxy};
use ribir_algo::Sc;
use rxrust::{ops::box_it::BoxOp, prelude::*};
use std::{
  cell::{Cell, RefCell},
  convert::Infallible,
  time::Instant,
};

use super::WriterControl;

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W> {
  data: Sc<RefCell<W>>,
  info: Sc<StatefulInfo>,
}

pub struct Reader<W>(Stateful<W>);

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

struct StatefulInfo {
  notifier: Notifier,
  /// The counter of the writer may be modified the data.
  writer_count: Cell<usize>,
  /// The batched modifies of the `State` which will be notified.
  batch_modified: Cell<ModifyScope>,
  /// The timestamp of the last modify of the data, not record the modify from
  /// the splitted or mapped state.
  last_modified: Cell<Instant>,
}

impl<W: 'static> StateReader for Stateful<W> {
  type Value = W;
  type OriginReader = Self;
  type Reader = Reader<W>;

  #[inline]
  fn read(&self) -> ReadRef<W> { ReadRef::new(self.data.borrow()) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { Reader(self.clone()) }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn time_stamp(&self) -> Instant { self.info.last_modified.get() }

  #[inline]
  fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self.info.notifier.raw_modifies()
  }

  fn try_into_value(self) -> Result<W, Self> {
    if self.data.ref_count() == 1 {
      let data = self.data.clone();
      drop(self);
      // SAFETY: `Rc::strong_count(&self.inner) == 1` guarantees unique access.
      let data = unsafe { Sc::try_unwrap(data).unwrap_unchecked() };
      Ok(data.into_inner())
    } else {
      Err(self)
    }
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
  fn clone_reader(&self) -> Self { self.0.clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn time_stamp(&self) -> Instant { self.0.time_stamp() }

  #[inline]
  fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.0.raw_modifies() }

  fn try_into_value(self) -> Result<Self::Value, Self> {
    let inner = self.0.clone_writer().0;
    drop(self);
    inner.try_into_value().map_err(Reader)
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
  fn time_stamp(&self) -> Instant { self.0.time_stamp() }

  #[inline]
  fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.0.raw_modifies() }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self> { self.0.try_into_value().map_err(Writer) }
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

impl WriterControl for Sc<StatefulInfo> {
  #[inline]
  fn last_modified_stamp(&self) -> &Cell<Instant> { &self.last_modified }

  #[inline]
  fn batched_modifies(&self) -> &Cell<ModifyScope> { &self.batch_modified }

  #[inline]
  fn notifier(&self) -> &Notifier { &self.notifier }

  #[inline]
  fn dyn_clone(&self) -> Box<dyn WriterControl> { Box::new(self.clone()) }
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
      let notifier = self.info.notifier.clone();
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
    if self.data.ref_count() == 1 {
      AppCtx::trigger_task(self.heap_ptr() as *const ());
    }
  }
}

impl<W> Writer<W> {
  #[inline]
  pub fn into_inner(self) -> Stateful<W> { self.0 }
}

macro_rules! compose_builder_impl {
  ($name: ident) => {
    impl<C: Compose + 'static> ComposeBuilder for $name<C> {
      #[inline]
      fn widget_build(self, ctx: &BuildCtx) -> Widget { Compose::compose(self).widget_build(ctx) }
    }

    impl<R: ComposeChild<Child = Option<C>> + 'static, C> ComposeChildBuilder for $name<R> {
      #[inline]
      fn widget_build(self, ctx: &BuildCtx) -> Widget {
        ComposeChild::compose_child(self, None).widget_build(ctx)
      }
    }
  };
}

compose_builder_impl!(Stateful);
compose_builder_impl!(Writer);

impl<R: Render> RenderBuilder for Stateful<R> {
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    match self.try_into_value() {
      Ok(r) => r.widget_build(ctx),
      Err(s) => {
        let w = RenderProxy::new(s.data.clone()).widget_build(ctx);
        w.dirty_subscribe(s.raw_modifies(), ctx)
      }
    }
  }
}

impl<R: Render> RenderBuilder for Writer<R> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self.0.widget_build(ctx) }
}

impl<R: Render> RenderBuilder for Reader<R> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self.0.clone_writer().widget_build(ctx) }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Self {
      data: Sc::new(RefCell::new(data)),
      info: Sc::new(StatefulInfo::new()),
    }
  }

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
  fn heap_ptr(&self) -> *const W { self.data.as_ptr() }

  fn write_ref(&self, scope: ModifyScope) -> WriteRef<'_, W> {
    let value = self.data.borrow_mut();
    WriteRef {
      value: Some(value),
      modified: false,
      modify_scope: scope,
      control: &self.info,
    }
  }

  fn writer_count(&self) -> usize { self.info.writer_count.get() }
  fn inc_writer(&self) { self.info.writer_count.set(self.writer_count() + 1); }
  fn dec_writer(&self) { self.info.writer_count.set(self.writer_count() - 1); }

  fn clone(&self) -> Self {
    self.inc_writer();
    Self {
      data: self.data.clone(),
      info: self.info.clone(),
    }
  }
}

impl StatefulInfo {
  fn new() -> Self {
    StatefulInfo {
      batch_modified: <_>::default(),
      writer_count: Cell::new(1),
      notifier: <_>::default(),
      last_modified: Cell::new(Instant::now()),
    }
  }
}

impl<W: SingleChild> SingleChild for Stateful<W> {}
impl<W: MultiChild> MultiChild for Stateful<W> {}

impl<W: SingleChild + Render> SingleParent for Stateful<W> {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    let p = self.widget_build(ctx);
    ctx.append_child(p.id(), child);
    p
  }
}

impl<W: MultiChild + Render> MultiParent for Stateful<W> {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    let p = self.widget_build(ctx);
    for c in children {
      ctx.append_child(p.id(), c);
    }
    p
  }
}

impl Notifier {
  pub(crate) fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self.0.clone().box_it()
  }

  pub(crate) fn next(&self, scope: ModifyScope) { self.0.clone().next(scope) }
}

impl<W: std::fmt::Debug> std::fmt::Debug for Stateful<W> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Stateful")
      .field(&*self.data.borrow())
      .finish()
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
      stateful.write().size = Size::new(100., 100.)
    }
    assert_eq!(stateful.read().size, Size::new(100., 100.));
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

  #[test]
  fn fix_pin_widget_node() {
    crate::reset_test_env!();

    let mut wnd = TestWindow::new(fn_widget!(MockBox { size: Size::new(100., 100.) }));
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
}
