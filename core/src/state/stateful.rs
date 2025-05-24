use std::{cell::RefCell, convert::Infallible};

use ahash::HashMap;
use ribir_algo::Sc;
use rxrust::ops::box_it::CloneableBoxOp;

use crate::prelude::*;

/// Stateful object use to watch the modifies of the inner data.
pub struct Stateful<W>(pub(crate) StateValueWriter<StateAtom<W>>);

pub struct StateValueWriter<D> {
  pub(crate) data: D,
  pub(crate) info: WriterInfo,
}

impl<D> StateValueWriter<D> {
  pub(crate) fn partial(self, id: &str) -> StateValueWriter<D> {
    let StateValueWriter { data, info } = self;
    let info = info.partials(id.to_string().into());
    StateValueWriter { data, info }
  }
}

struct PartialMgr {
  partial_notifiers: HashMap<Vec<CowArc<str>>, Subject>,
}

impl PartialMgr {
  fn new(root: Subject) -> Self {
    let mut m = HashMap::default();
    m.insert(vec![], root);
    Self { partial_notifiers: m }
  }

  fn restrict_partial(&mut self, path: Vec<CowArc<str>>) -> Subject {
    self
      .partial_notifiers
      .entry(path)
      .or_default()
      .clone()
  }

  fn partial(&self, path: &[CowArc<str>]) -> Option<&Subject> { self.partial_notifiers.get(path) }

  fn unrestrict(&mut self, id: &Vec<CowArc<str>>) { self.partial_notifiers.remove(id); }
}

pub type Reader<W> = StateValueReader<StateAtom<W>>;

pub struct StateValueReader<D>(pub(crate) D);

impl<V: ?Sized, W: 'static> StateReader for StateValueReader<W>
where
  W: StateValue<Value = V> + Clone,
{
  type Value = V;
  type Reader = StateValueReader<W>;

  fn clone_reader(&self) -> Self::Reader { StateValueReader(self.0.clone()) }

  fn read(&self) -> ReadRef<V> { self.0.read() }

  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }
}

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
  scope: ModifyScope,
  partial: Option<PartialPath>,
}

impl ModifyInfo {
  pub fn new(scope: ModifyScope, partial: Option<PartialPath>) -> Self { Self { scope, partial } }

  pub fn contains(&self, scope: ModifyScope) -> bool { self.scope.contains(scope) }

  pub fn scope(&self) -> ModifyScope { self.scope }

  pub fn is_from_partial(&self) -> bool { self.partial.is_some() }

  pub fn partial_path(&self) -> Option<&PartialPath> { self.partial.as_ref() }
}

#[derive(Default)]
pub struct WriterInfoInner {
  notifier: Notifier,
  /// The batched modifies of the `State` which will be notified.
  batched_modifies: ModifyScope,
}

#[derive(Default)]
pub struct WriterInfo(Sc<RefCell<WriterInfoInner>>);

type Subject = rxrust::prelude::Subject<'static, ModifyInfo, Infallible>;

enum Notifier {
  Simple(Subject),
  Partials { partials: Sc<RefCell<PartialMgr>>, path: Vec<CowArc<str>> },
}

impl Default for Notifier {
  fn default() -> Self { Notifier::Simple(Subject::default()) }
}

impl Notifier {
  pub(crate) fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.notifier().box_it()
  }

  #[inline]
  fn notifier(&self) -> Subject {
    match self {
      Notifier::Simple(n) => n.clone(),
      Notifier::Partials { partials, path } => partials
        .borrow_mut()
        .restrict_partial(path.clone()),
    }
  }

  pub(crate) fn unregister_writer(&self) -> impl Subscription {
    let notifier = self.notifier();
    if let Notifier::Partials { partials, path: id } = self {
      partials.borrow_mut().unrestrict(id)
    }
    notifier
  }

  pub(crate) fn notify_modifies(&self, scope: ModifyScope) {
    match self {
      Notifier::Simple(n) => n.clone().next(ModifyInfo::new(scope, None)),
      Notifier::Partials { partials, path } => {
        for i in 0..=path.len() {
          if let Some(s) = partials.borrow_mut().partial(&path[0..i]) {
            let child_path =
              if i == path.len() { None } else { Some(path[i..path.len()].to_vec()) };
            s.clone().next(ModifyInfo::new(scope, child_path));
          }
        }
      }
    }
  }

  fn partial(&mut self, id: CowArc<str>) -> Notifier {
    if matches!(self, Notifier::Simple(_)) {
      let notifier = self.notifier();

      *self = Notifier::Partials {
        partials: Sc::new(RefCell::new(PartialMgr::new(notifier))),
        path: vec![],
      };
    }

    match self {
      Notifier::Partials { partials, path: origin_path } => {
        let mut path = origin_path.clone();
        path.push(id);
        Notifier::Partials { partials: partials.clone(), path }
      }
      _ => unreachable!(),
    }
  }
}

impl<W: ?Sized, D> StateReader for StateValueWriter<D>
where
  D: StateValue<Value = W> + 'static + Clone,
{
  type Value = W;
  type Reader = StateValueReader<D>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.data.read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { StateValueReader(self.data.clone()) }

  fn try_into_value(self) -> Result<W, Self>
  where
    W: Sized,
  {
    let Self { data, info } = self;
    data
      .try_into_value()
      .map_err(|data| Self { data, info })
  }
}

impl<W: ?Sized, D> StateWatcher for StateValueWriter<D>
where
  D: StateValue<Value = W> + 'static + Clone,
{
  type Watcher = Watcher<Self::Reader>;

  fn into_reader(self) -> Result<Self::Reader, Self> {
    if self.info.ref_count() == 1 { Ok(self.clone_reader()) } else { Err(self) }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.info.raw_modifies()
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<W: ?Sized, D> StateWriter for StateValueWriter<D>
where
  D: StateValue<Value = W> + 'static + Clone,
{
  type State = D;
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
  fn clone_writer(&self) -> Self { Self { data: self.data.clone(), info: self.info.clone() } }

  fn part_writer<V: ?Sized + 'static, M>(
    &self, id: Option<&str>, part_map: M,
  ) -> StateValueWriter<MapState<Self::State, M>>
  where
    M: Fn(&mut Self::Value) -> PartMut<V> + Clone + 'static,
    Self: Sized,
  {
    let w = StateValueWriter { data: self.data.clone().map(part_map), info: self.info.clone() };
    if let Some(id) = id { w.partial(id) } else { w }
  }
}

impl<V: 'static> StateReader for Stateful<V> {
  type Value = V;
  type Reader = Reader<V>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.0.read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  fn clone_reader(&self) -> Self::Reader
  where
    Self: Sized,
  {
    self.0.clone_reader()
  }
}

impl<V: 'static> StateWatcher for Stateful<V> {
  type Watcher = Watcher<Reader<V>>;

  fn into_reader(self) -> Result<Reader<V>, Self> { self.0.into_reader().map_err(|d| Stateful(d)) }

  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  fn clone_watcher(&self) -> Watcher<Reader<V>> {
    Watcher::new(self.clone_reader(), self.0.raw_modifies())
  }

  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.0.raw_modifies()
  }
}

impl<V: 'static> StateWriter for Stateful<V> {
  type State = StateAtom<V>;
  #[inline]
  fn write(&self) -> WriteRef<V> { self.0.write() }

  #[inline]
  fn silent(&self) -> WriteRef<V> { self.0.silent() }

  #[inline]
  fn shallow(&self) -> WriteRef<V> { self.0.shallow() }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  fn clone_writer(&self) -> Self { Stateful(self.0.clone_writer()) }

  fn part_writer<W: ?Sized + 'static, M>(
    &self, id: Option<&str>, part_map: M,
  ) -> StateValueWriter<MapState<Self::State, M>>
  where
    M: Fn(&mut Self::Value) -> PartMut<W> + Clone + 'static,
    Self: Sized,
  {
    let data = StateValue::map(self.0.data.clone(), part_map);
    let w = StateValueWriter { data, info: self.0.info.clone() };
    if let Some(id) = id { w.partial(id) } else { w }
  }
}

impl WriterInfo {
  pub(crate) fn clone(&self) -> Self { Self(self.0.clone()) }

  pub(crate) fn partials(&self, id: CowArc<str>) -> WriterInfo {
    WriterInfo(Sc::new(RefCell::new(WriterInfoInner {
      notifier: self.0.borrow_mut().notifier.partial(id),
      batched_modifies: ModifyScope::empty(),
    })))
  }

  pub(crate) fn ref_count(&self) -> usize { self.0.ref_count() }

  pub(crate) fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.0.borrow().notifier.raw_modifies().box_it()
  }

  pub(crate) fn modified(&self, scope: ModifyScope) {
    self.0.borrow_mut().batched_modifies |= scope;
  }

  pub(crate) fn has_modified(&self) -> bool { self.0.borrow().batched_modifies.is_empty() }

  pub(crate) fn submit_modified(&self) {
    let scope = self.0.borrow().batched_modifies;
    self.0.borrow_mut().batched_modifies = ModifyScope::empty();
    self.0.borrow().notifier.notify_modifies(scope);
  }

  fn unregister_writer(&self) -> impl Subscription { self.0.borrow().notifier.unregister_writer() }
}

impl Drop for WriterInfo {
  fn drop(&mut self) {
    if self.0.ref_count() == 1 {
      let notifier = self.unregister_writer();

      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      let _ = AppCtx::spawn_local(async move { notifier.unsubscribe() });
    }
  }
}

impl<W> Stateful<W> {
  pub fn new(data: W) -> Self {
    Self(StateValueWriter { data: StateAtom::new(data), info: WriterInfo::default() })
  }

  /// Determines if two `Stateful` instances point to the same underlying data.
  ///
  /// Performs pointer equality checks on both:
  /// - The state data container
  /// - The associated metadata
  #[inline]
  pub fn ptr_eq(this: &Self, other: &Self) -> bool {
    this.0.data.is_same_obj(&other.0.data) && Sc::ptr_eq(&this.0.info.0, &other.0.info.0)
  }

  pub fn from_pipe(p: impl Pipe<Value = W>) -> (Self, BoxSubscription<'static>)
  where
    Self: 'static,
  {
    let (v, p) = p.unzip(ModifyScope::DATA, None);
    let s = Stateful::new(v);
    let s2 = s.0.clone_writer();
    let u = p.subscribe(move |(_, v)| *s2.write() = v);
    (s, u)
  }
}

impl<W: ?Sized, D> StateValueWriter<D>
where
  D: StateValue<Value = W>,
{
  fn write_ref(&self, scope: ModifyScope) -> WriteRef<'_, W> {
    let value = self.data.write();
    WriteRef { value, modified: false, modify_scope: scope, info: &self.info }
  }
}

impl<W: std::fmt::Debug + 'static> std::fmt::Debug for Stateful<W> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Stateful")
      .field(&*self.0.data.read())
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
    assert_eq!(v.0.data.ref_count(), 2);
    assert_eq!(v.0.info.ref_count(), 1);
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
    assert_eq!(v.0.data.ref_count(), 2);
    assert_eq!(v.0.info.ref_count(), 1);
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
    assert_eq!(v.0.data.ref_count(), 2);
    assert_eq!(v.0.info.ref_count(), 1);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn render_in_downstream_no_circle() {}

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn writer_in_downstream_unsubscribe() {
    crate::reset_test_env!();

    let v = Stateful::new(1);
    let data = v.0.data.clone();
    let notifier = v.0.info.0.borrow().notifier.notifier();
    {
      let info = v.0.info.clone();
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
