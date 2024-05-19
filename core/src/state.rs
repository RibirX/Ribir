mod map_state;
mod prior_op;
mod splitted_state;
mod stateful;
mod watcher;
use std::{
  cell::{Cell, UnsafeCell},
  convert::Infallible,
  mem::MaybeUninit,
  ops::DerefMut,
};
pub mod state_cell;

pub use map_state::*;
pub use prior_op::*;
use rxrust::ops::box_it::{BoxOp, CloneableBoxOp};
pub use splitted_state::*;
pub use state_cell::PartData;
use state_cell::{StateCell, ValueMutRef, ValueRef};
pub use stateful::*;
pub use watcher::*;

use crate::prelude::*;

/// The `StateReader` trait allows for reading, clone and map the state.
pub trait StateReader: 'static {
  /// The value type of this state.
  type Value;
  /// The origin state type that this state map or split from . Otherwise
  /// return itself.
  type OriginReader: StateReader;
  type Reader: StateReader<Value = Self::Value>;

  /// Return a reference of this state.
  fn read(&self) -> ReadRef<Self::Value>;
  /// get a clone of this state that only can read.
  fn clone_reader(&self) -> Self::Reader;
  /// Maps an reader to another by applying a function to a contained
  /// value. The return reader is just a shortcut to access part of the origin
  /// reader.
  ///
  /// Note, `MapReader` is a shortcut to access a portion of the original
  /// reader. It's assumed that the `map` function returns a part of the
  /// original data, not a cloned portion. Otherwise, the returned reader will
  /// not respond to state changes.
  #[inline]
  fn map_reader<U, F>(&self, map: F) -> MapReader<Self::Reader, F>
  where
    F: Fn(&Self::Value) -> PartData<U> + Clone,
  {
    MapReader { origin: self.clone_reader(), part_map: map }
  }
  /// Return the origin reader that this state map or split from . Otherwise
  /// return itself.
  fn origin_reader(&self) -> &Self::OriginReader;

  /// try convert this state into the value, if there is no other share this
  /// state, otherwise return an error with self.
  fn try_into_value(self) -> Result<Self::Value, Self>
  where
    Self: Sized,
    Self::Value: Sized;
}

pub trait StateWatcher: StateReader {
  /// Return a modifies `Rx` stream of the state, user can subscribe it to
  /// response the state changes.
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self
      .raw_modifies()
      .filter(|s| s.contains(ModifyScope::DATA))
      .box_it()
  }

  /// Return a modifies `Rx` stream of the state, including all modifies. Use
  /// `modifies` instead if you only want to response the data changes.
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible>;

  /// Clone a reader that can be used to observe the modifies of the state.
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

pub trait StateWriter: StateWatcher {
  type Writer: StateWriter<Value = Self::Value>;
  type OriginWriter: StateWriter;

  /// Return a write reference of this state.
  fn write(&self) -> WriteRef<Self::Value>;
  /// Return a silent write reference which notifies will be ignored by the
  /// framework.
  fn silent(&self) -> WriteRef<Self::Value>;
  /// Return a shallow write reference. Modify across this reference will notify
  /// framework only. That means the modifies on shallow reference should only
  /// effect framework but not effect on data. eg. temporary to modify the
  /// state and then modifies it back to trigger the view update. Use it only
  /// if you know how a shallow reference works.
  fn shallow(&self) -> WriteRef<Self::Value>;
  /// Clone this state writer.
  fn clone_writer(&self) -> Self::Writer;
  /// Return the origin writer that this state map or split from.
  fn origin_writer(&self) -> &Self::OriginWriter;
  /// Return a new writer that be part of the origin writer by applying a
  /// function to the contained value.
  ///
  /// The return writer share the same data with the origin writer. But modify
  /// the data through the return writer will not trigger the views depend on
  /// the origin writer to update.
  ///
  /// If you want a new writer that has same notifier with the origin writer,
  /// you can use `map_writer(...)`.
  ///
  /// ##Notice
  ///
  /// The `mut_map` function accepts a mutable reference, but it should not be
  /// modified. Instead, return a mutable reference to a part of it. Ribir
  /// uses this to access a portion of the original data, so a read operation
  /// may also call it. Ribir assumes that the original data will not be
  /// modified within this function. Therefore, if the original data is
  /// modified, no downstream will be notified.
  #[inline]
  fn split_writer<V, W>(&self, mut_map: W) -> SplittedWriter<Self::Writer, W>
  where
    W: Fn(&mut Self::Value) -> PartData<V> + Clone + 'static,
  {
    SplittedWriter::new(self.clone_writer(), mut_map)
  }

  /// Return a new writer by applying a function to the contained value. The
  /// return writer is just a shortcut to access part of the origin writer.
  ///
  /// ##Notice
  ///
  /// The `mut_map` function accepts a mutable reference, but it should not be
  /// modified. Instead, return a mutable reference to a part of it. Ribir
  /// uses this to access a portion of the original data, so a read operation
  /// may also call it. Ribir assumes that the original data will not be
  /// modified within this function. Therefore, if the original data is
  /// modified, no downstream will be notified.
  #[inline]
  fn map_writer<V, M>(&self, part_map: M) -> MapWriter<Self::Writer, M>
  where
    M: Fn(&mut Self::Value) -> PartData<V> + Clone,
  {
    let origin = self.clone_writer();
    MapWriter { origin, part_map }
  }
}

/// Wraps a borrowed reference to a value in a state.
/// A wrapper type for an immutably borrowed value from a `StateReader`.
pub struct ReadRef<'a, V>(ValueRef<'a, V>);

pub struct WriteRef<'a, V> {
  value: ValueMutRef<'a, V>,
  control: &'a dyn WriterControl,
  modify_scope: ModifyScope,
  modified: bool,
}

/// Enum to store both stateless and stateful object.
pub struct State<W>(pub(crate) UnsafeCell<InnerState<W>>);

pub(crate) enum InnerState<W> {
  Data(StateCell<W>),
  Stateful(Stateful<W>),
}

trait WriterControl {
  fn batched_modifies(&self) -> &Cell<ModifyScope>;
  fn notifier(&self) -> &Notifier;
  fn dyn_clone(&self) -> Box<dyn WriterControl>;
}

impl<T: 'static> StateReader for State<T> {
  type Value = T;
  type OriginReader = Self;
  type Reader = Reader<T>;

  fn read(&self) -> ReadRef<T> {
    match self.inner_ref() {
      InnerState::Data(w) => ReadRef::new(w.read()),
      InnerState::Stateful(w) => w.read(),
    }
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.as_stateful().clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  fn try_into_value(self) -> Result<Self::Value, Self> {
    match self.0.into_inner() {
      InnerState::Data(w) => Ok(w.into_inner()),
      InnerState::Stateful(w) => w.try_into_value().map_err(State::stateful),
    }
  }
}

impl<T: 'static> StateWatcher for State<T> {
  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.as_stateful().raw_modifies()
  }
}

impl<T: 'static> StateWriter for State<T> {
  type Writer = Writer<T>;
  type OriginWriter = Self;

  #[inline]
  fn write(&self) -> WriteRef<T> { self.as_stateful().write() }

  #[inline]
  fn silent(&self) -> WriteRef<T> { self.as_stateful().silent() }

  #[inline]
  fn shallow(&self) -> WriteRef<T> { self.as_stateful().shallow() }

  #[inline]
  fn clone_writer(&self) -> Self::Writer { self.as_stateful().clone_writer() }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { self }
}

impl<W> State<W> {
  pub fn stateful(stateful: Stateful<W>) -> Self {
    State(UnsafeCell::new(InnerState::Stateful(stateful)))
  }

  pub fn value(value: W) -> Self { State(UnsafeCell::new(InnerState::Data(StateCell::new(value)))) }

  pub fn as_stateful(&self) -> &Stateful<W> {
    match self.inner_ref() {
      InnerState::Data(w) => {
        assert!(w.is_unused());

        let mut uninit: MaybeUninit<_> = MaybeUninit::uninit();
        // Safety: we already check there is no other reference to the state data.
        unsafe {
          std::ptr::copy(w, uninit.as_mut_ptr(), 1);
          let value = uninit.assume_init().into_inner();
          let stateful = InnerState::Stateful(Stateful::new(value));
          let copy = std::mem::replace(&mut *self.0.get(), stateful);
          // this is a copy of the inner data so we need forget it.
          std::mem::forget(copy);
        };

        match self.inner_ref() {
          InnerState::Stateful(w) => w,
          _ => unreachable!(),
        }
      }
      InnerState::Stateful(w) => w,
    }
  }

  fn inner_ref(&self) -> &InnerState<W> {
    // Safety: we only use this method to get the inner state, and no way to get the
    // mutable reference of the inner state except the `as_stateful` method and the
    // `as_stateful` will check the inner borrow state.
    unsafe { &*self.0.get() }
  }
}

impl<'a, V> ReadRef<'a, V> {
  pub(crate) fn new(r: ValueRef<'a, V>) -> ReadRef<'a, V> { ReadRef(r) }

  /// Make a new `ReadRef` by mapping the value of the current `ReadRef`.
  pub fn map<U>(mut r: ReadRef<'a, V>, f: impl FnOnce(&V) -> PartData<U>) -> ReadRef<'a, U> {
    ReadRef(ValueRef { value: f(&mut r.0.value), borrow: r.0.borrow })
  }

  /// Split the current `ReadRef` into two `ReadRef`s by mapping the value to
  /// two parts.
  pub fn map_split<U, W>(
    orig: ReadRef<'a, V>, f: impl FnOnce(&V) -> (PartData<U>, PartData<W>),
  ) -> (ReadRef<'a, U>, ReadRef<'a, W>) {
    let (a, b) = f(&*orig);
    let borrow = orig.0.borrow.clone();

    (ReadRef(ValueRef { value: a, borrow: borrow.clone() }), ReadRef(ValueRef { value: b, borrow }))
  }

  pub(crate) fn mut_as_ref_map<U>(
    orig: ReadRef<'a, V>, f: impl FnOnce(&mut V) -> PartData<U>,
  ) -> ReadRef<'a, U> {
    let ValueRef { value, borrow } = orig.0;
    let value = match value {
      PartData::PartRef(mut ptr) => unsafe {
        // Safety: This method is used to map a state to a part of it. Although a `&mut
        // T` is passed to the closure, it is the user's responsibility to
        // ensure that the closure does not modify the state.
        f(ptr.as_mut())
      },
      PartData::PartData(mut data) => f(&mut data),
    };

    ReadRef(ValueRef { value, borrow })
  }
}

impl<'a, V> WriteRef<'a, V> {
  pub fn map<U, M>(mut orig: WriteRef<'a, V>, part_map: M) -> WriteRef<'a, U>
  where
    M: Fn(&mut V) -> PartData<U>,
  {
    let value = part_map(&mut orig.value);
    let borrow = orig.value.borrow.clone();
    let value = ValueMutRef { value, borrow };

    WriteRef { value, modified: false, modify_scope: orig.modify_scope, control: orig.control }
  }

  pub fn map_split<U1, U2, F>(
    mut orig: WriteRef<'a, V>, f: F,
  ) -> (WriteRef<'a, U1>, WriteRef<'a, U2>)
  where
    F: FnOnce(&mut V) -> (PartData<U1>, PartData<U2>),
  {
    let WriteRef { control, modify_scope, modified, .. } = orig;
    let (a, b) = f(&mut *orig.value);
    let borrow = orig.value.borrow.clone();
    let a = ValueMutRef { value: a, borrow: borrow.clone() };
    let b = ValueMutRef { value: b, borrow };
    (
      WriteRef { value: a, modified, modify_scope, control },
      WriteRef { value: b, modified, modify_scope, control },
    )
  }

  /// Forget all modifies of this reference. So all the modifies occurred on
  /// this reference before this call will not be notified. Return true if there
  /// is any modifies on this reference.
  #[inline]
  pub fn forget_modifies(&mut self) -> bool { std::mem::replace(&mut self.modified, false) }
}

impl<'a, W> Deref for ReadRef<'a, W> {
  type Target = W;

  #[track_caller]
  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a, W> Deref for WriteRef<'a, W> {
  type Target = W;
  #[track_caller]
  #[inline]
  fn deref(&self) -> &Self::Target { self.value.deref() }
}

impl<'a, W> DerefMut for WriteRef<'a, W> {
  #[track_caller]
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.modified = true;
    self.value.deref_mut()
  }
}

impl<'a, W> Drop for WriteRef<'a, W> {
  fn drop(&mut self) {
    let Self { control, modify_scope, modified, .. } = self;
    if !*modified {
      return;
    }

    let batched_modifies = control.batched_modifies();
    if batched_modifies.get().is_empty() && !modify_scope.is_empty() {
      batched_modifies.set(*modify_scope);

      let control = control.dyn_clone();
      let _ = AppCtx::spawn_local(async move {
        let scope = control
          .batched_modifies()
          .replace(ModifyScope::empty());
        control.notifier().next(scope);
      });
    } else {
      batched_modifies.set(*modify_scope | batched_modifies.get());
    }
  }
}

// todo: We should use `BoxPipe<T>` to replace `State<T>` as the dynamic child.
// remove it after no widget use `State<T>` Child.
pub(crate) trait StateFrom<V> {
  fn state_from(value: V) -> Self;
}

impl<W> StateFrom<W> for State<W> {
  #[inline]
  fn state_from(value: W) -> State<W> { State::value(value) }
}

impl<W> StateFrom<Stateful<W>> for State<W> {
  #[inline]
  fn state_from(value: Stateful<W>) -> State<W> { State::stateful(value) }
}

impl<W, T> From<T> for State<W>
where
  Self: StateFrom<T>,
{
  fn from(value: T) -> Self { StateFrom::state_from(value) }
}

impl<C: Compose + 'static> ComposeBuilder for State<C> {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { Compose::compose(self).build(ctx) }
}

impl<P: ComposeChild<Child = Option<C>> + 'static, C> ComposeChildBuilder for State<P> {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { ComposeChild::compose_child(self, None).build(ctx) }
}

impl<W: SingleChild> SingleChild for State<W> {}
impl<W: MultiChild> MultiChild for State<W> {}

impl<R: Render> RenderBuilder for State<R> {
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget {
    match self.0.into_inner() {
      InnerState::Data(w) => w.into_inner().build(ctx),
      InnerState::Stateful(w) => w.build(ctx),
    }
  }
}

impl<W: SingleChild + Render> SingleParent for State<W> {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    match self.0.into_inner() {
      InnerState::Data(w) => w.into_inner().compose_child(child, ctx),
      InnerState::Stateful(w) => w.compose_child(child, ctx),
    }
  }
}

impl<W: MultiChild + Render> MultiParent for State<W> {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    match self.0.into_inner() {
      InnerState::Data(w) => w.into_inner().compose_children(children, ctx),
      InnerState::Stateful(w) => w.compose_children(children, ctx),
    }
  }
}

impl<T: StateReader + 'static> Query for T
where
  T::Value: 'static + Sized,
{
  #[inline]
  fn query_inside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.query_outside_first(type_id, callback)
  }

  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    let any: &T::Value = &self.read();
    if type_id == any.type_id() { callback(any) } else { true }
  }
}

macro_rules! impl_compose_builder {
  ($name:ident) => {
    impl<V, W, WM> ComposeBuilder for $name<W, WM>
    where
      W: StateWriter,
      WM: Fn(&mut W::Value) -> PartData<V> + Clone + 'static,
      V: Compose + 'static,
    {
      fn build(self, ctx: &crate::context::BuildCtx) -> Widget { Compose::compose(self).build(ctx) }
    }

    impl<V, W, WM, Child> ComposeChildBuilder for $name<W, WM>
    where
      W: StateWriter,
      WM: Fn(&mut W::Value) -> PartData<V> + Clone + 'static,
      V: ComposeChild<Child = Option<Child>> + 'static,
    {
      #[inline]
      fn build(self, ctx: &BuildCtx) -> Widget {
        ComposeChild::compose_child(self, None).build(ctx)
      }
    }
  };
}

impl_compose_builder!(MapWriter);
impl_compose_builder!(SplittedWriter);

#[cfg(test)]
mod tests {
  use ribir_algo::Sc;

  use super::*;
  #[cfg(target_arch = "wasm32")]
  use crate::test_helper::wasm_bindgen_test;
  use crate::{reset_test_env, timer::Timer};

  struct Origin {
    a: i32,
    b: i32,
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn map_same_with_origin() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let map_state = origin.map_writer(|v| PartData::from_ref_mut(&mut v.b));

    let track_origin = Sc::new(Cell::new(0));
    let track_map = Sc::new(Cell::new(0));

    let c_origin = track_origin.clone();
    origin.modifies().subscribe(move |_| {
      c_origin.set(c_origin.get() + 1);
    });

    let c_map = track_map.clone();
    map_state.modifies().subscribe(move |_| {
      c_map.set(c_map.get() + 1);
    });

    origin.write().a = 1;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), 1);
    assert_eq!(track_map.get(), 1);

    *map_state.write() = 1;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), 2);
    assert_eq!(track_map.get(), 2);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn split_not_notify_origin() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let split = origin.split_writer(|v| PartData::from_ref_mut(&mut v.b));

    let track_origin = Sc::new(Cell::new(0));
    let track_split = Sc::new(Cell::new(0));

    let c_origin = track_origin.clone();
    origin.modifies().subscribe(move |s| {
      c_origin.set(c_origin.get() + s.bits());
    });

    let c_split = track_split.clone();
    split.modifies().subscribe(move |s| {
      c_split.set(c_split.get() + s.bits());
    });

    *split.write() = 0;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), ModifyScope::DATA.bits());
    assert_eq!(track_split.get(), ModifyScope::BOTH.bits());

    origin.write().b = 0;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(track_origin.get(), ModifyScope::DATA.bits() + ModifyScope::BOTH.bits());
    // splitted downstream will not be notified.
    assert_eq!(track_split.get(), ModifyScope::BOTH.bits());
  }

  struct C;

  impl Compose for C {
    fn compose(_: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
      fn_widget! { Void }
    }
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn state_writer_compose_builder() {
    reset_test_env!();

    let _state_compose_widget = fn_widget! {
      State::value(C)
    };

    let _sateful_compose_widget = fn_widget! {
      Stateful::new(C)
    };

    let _writer_compose_widget = fn_widget! {
      Stateful::new(C).clone_writer()
    };

    let _map_writer_compose_widget = fn_widget! {
      Stateful::new((C, 0))
        .map_writer(|v| PartData::from_ref_mut(&mut v.0))
    };
    let _split_writer_compose_widget = fn_widget! {
      Stateful::new((C, 0))
        .split_writer(|v| PartData::from_ref_mut(&mut v.0))
    };
  }

  struct CC;
  impl ComposeChild for CC {
    type Child = Option<Widget>;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
      fn_widget! { @{ Void } }
    }
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn state_writer_compose_child_builder() {
    reset_test_env!();

    let _state_with_child = fn_widget! {
      let cc = State::value(CC);
      @$cc { @{ Void } }
    };

    let _state_without_child = fn_widget! {
      State::value(CC)
    };

    let _stateful_with_child = fn_widget! {
      let cc = Stateful::new(CC);
      @$cc { @{ Void } }
    };

    let _stateful_without_child = fn_widget! {
      Stateful::new(CC)
    };

    let _writer_with_child = fn_widget! {
      let cc = Stateful::new(CC).clone_writer();
      @$cc { @{ Void } }
    };

    let _writer_without_child = fn_widget! {
      Stateful::new(CC).clone_writer()
    };

    let _map_writer_with_child = fn_widget! {
      let w = Stateful::new((CC, 0))
        .map_writer(|v| PartData::from_ref_mut(&mut v.0));
      @$w { @{ Void } }
    };

    let _map_writer_without_child = fn_widget! {
      Stateful::new((CC, 0))
        .map_writer(|v| PartData::from_ref_mut(&mut v.0))
    };

    let _split_writer_with_child = fn_widget! {
      let w = Stateful::new((CC, 0))
        .split_writer(|v| PartData::from_ref_mut(&mut v.0));
      @$w { @{ Void } }
    };

    let _split_writer_without_child = fn_widget! {
      Stateful::new((CC, 0))
        .split_writer(|v| PartData::from_ref_mut(&mut v.0))
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn state_reader_builder() {
    reset_test_env!();

    let _state_render_widget = fn_widget! {
      State::value(Void)
    };

    let _stateful_render_widget = fn_widget! {
      Stateful::new(Void)
    };

    let _writer_render_widget = fn_widget! {
      Stateful::new(Void).clone_writer()
    };

    let _map_reader_render_widget = fn_widget! {
      Stateful::new((Void, 0)).map_reader(|v| PartData::from_ref(&v.0))
    };

    let _map_writer_render_widget = fn_widget! {
      Stateful::new((Void, 0))
        .map_writer(|v| PartData::from_ref_mut(&mut v.0))
    };

    let _split_writer_render_widget = fn_widget! {
      Stateful::new((Void, 0))
        .split_writer(|v| PartData::from_ref_mut(&mut v.0))
    };
  }
}
