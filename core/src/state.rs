mod map_state;
mod prior_op;
mod splitted_state;
mod stateful;
mod watcher;
use std::{cell::UnsafeCell, convert::Infallible, mem::MaybeUninit, ops::DerefMut};
pub mod state_cell;

pub use map_state::*;
pub use prior_op::*;
use ribir_algo::Sc;
use rxrust::ops::box_it::{BoxOp, CloneableBoxOp};
pub use splitted_state::*;
pub use state_cell::{PartData, ReadRef};
use state_cell::{StateCell, ValueMutRef};
pub use stateful::*;
pub use watcher::*;

use crate::{prelude::*, render_helper::RenderProxy};

/// The `StateReader` trait allows for reading, clone and map the state.
pub trait StateReader: 'static {
  /// The value type of this state.
  type Value: ?Sized;
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
  fn map_reader<U: ?Sized, F>(&self, map: F) -> MapReader<Self::Reader, F>
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

  /// Convert the writer to a reader if no other writers exist.
  fn into_reader(self) -> Result<Self::Reader, Self>
  where
    Self: Sized;

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
  fn split_writer<V: ?Sized, W>(&self, mut_map: W) -> SplittedWriter<Self::Writer, W>
  where
    W: Fn(&mut Self::Value) -> PartData<V> + Clone,
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
  fn map_writer<V: ?Sized, M>(&self, part_map: M) -> MapWriter<Self::Writer, M>
  where
    M: Fn(&mut Self::Value) -> PartData<V> + Clone,
  {
    let origin = self.clone_writer();
    MapWriter { origin, part_map }
  }
}

pub struct WriteRef<'a, V: ?Sized> {
  value: ValueMutRef<'a, V>,
  info: &'a Sc<WriterInfo>,
  modify_scope: ModifyScope,
  modified: bool,
}

/// Enum to store both stateless and stateful object.
pub struct State<W>(pub(crate) UnsafeCell<InnerState<W>>);

pub(crate) enum InnerState<W> {
  Data(StateCell<W>),
  Stateful(Stateful<W>),
}

impl<T: 'static> StateReader for State<T> {
  type Value = T;
  type OriginReader = Self;
  type Reader = Reader<T>;

  fn read(&self) -> ReadRef<T> {
    match self.inner_ref() {
      InnerState::Data(w) => w.read(),
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
  type Writer = Stateful<T>;
  type OriginWriter = Self;

  fn into_reader(self) -> Result<Self::Reader, Self>
  where
    Self: Sized,
  {
    match self.0.into_inner() {
      InnerState::Data(d) => Ok(Reader(Sc::new(d))),
      InnerState::Stateful(s) => s.into_reader().map_err(State::stateful),
    }
  }

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

impl<'a, V: ?Sized> WriteRef<'a, V> {
  pub fn map<U: ?Sized, M>(mut orig: WriteRef<'a, V>, part_map: M) -> WriteRef<'a, U>
  where
    M: Fn(&mut V) -> PartData<U>,
  {
    let inner = part_map(&mut orig.value);
    let borrow = orig.value.borrow.clone();
    let value = ValueMutRef { inner, borrow };

    WriteRef { value, modified: false, modify_scope: orig.modify_scope, info: orig.info }
  }

  /// Makes a new `WriteRef` for an optional component of the borrowed data. The
  /// original guard is returned as an `Err(..)` if the closure returns
  /// `None`.
  ///
  /// This is an associated function that needs to be used as
  /// `WriteRef::filter_map(...)`. A method would interfere with methods of the
  /// same name on `T` used through `Deref`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let c = Stateful::new(vec![1, 2, 3]);
  /// let b1: WriteRef<'_, Vec<u32>> = c.write();
  /// let b2: Result<WriteRef<'_, u32>, _> =
  ///   WriteRef::filter_map(b1, |v| v.get(1).map(PartData::from_ref));
  /// assert_eq!(*b2.unwrap(), 2);
  /// ```
  pub fn filter_map<U: ?Sized, M>(
    mut orig: WriteRef<'a, V>, part_map: M,
  ) -> Result<WriteRef<'a, U>, Self>
  where
    M: Fn(&mut V) -> Option<PartData<U>>,
  {
    match part_map(&mut orig.value) {
      Some(inner) => {
        let borrow = orig.value.borrow.clone();
        let value = ValueMutRef { inner, borrow };
        let WriteRef { modify_scope, info, .. } = orig;

        Ok(WriteRef { value, modified: false, modify_scope, info })
      }
      None => Err(orig),
    }
  }

  pub fn map_split<U1: ?Sized, U2: ?Sized, F>(
    mut orig: WriteRef<'a, V>, f: F,
  ) -> (WriteRef<'a, U1>, WriteRef<'a, U2>)
  where
    F: FnOnce(&mut V) -> (PartData<U1>, PartData<U2>),
  {
    let WriteRef { info, modify_scope, modified, .. } = orig;
    let (a, b) = f(&mut *orig.value);
    let borrow = orig.value.borrow.clone();
    let a = ValueMutRef { inner: a, borrow: borrow.clone() };
    let b = ValueMutRef { inner: b, borrow };
    (
      WriteRef { value: a, modified, modify_scope, info },
      WriteRef { value: b, modified, modify_scope, info },
    )
  }

  /// Forget all modifies of this reference. So all the modifies occurred on
  /// this reference before this call will not be notified. Return true if there
  /// is any modifies on this reference.
  #[inline]
  pub fn forget_modifies(&mut self) -> bool { std::mem::replace(&mut self.modified, false) }
}

impl<'a, W: ?Sized> Deref for WriteRef<'a, W> {
  type Target = W;
  #[track_caller]
  #[inline]
  fn deref(&self) -> &Self::Target { self.value.deref() }
}

impl<'a, W: ?Sized> DerefMut for WriteRef<'a, W> {
  #[track_caller]
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.modified = true;
    self.value.deref_mut()
  }
}

impl<'a, W: ?Sized> Drop for WriteRef<'a, W> {
  fn drop(&mut self) {
    let Self { info, modify_scope, modified, .. } = self;
    if !*modified {
      return;
    }

    let batched_modifies = &info.batched_modifies;
    if batched_modifies.get().is_empty() && !modify_scope.is_empty() {
      batched_modifies.set(*modify_scope);

      let info = info.clone();
      let _ = AppCtx::spawn_local(async move {
        let scope = info
          .batched_modifies
          .replace(ModifyScope::empty());
        info.notifier.next(scope);
      });
    } else {
      batched_modifies.set(*modify_scope | batched_modifies.get());
    }
  }
}

pub(crate) struct WriterRender<T>(pub(crate) T);

struct ReaderRender<T>(pub(crate) T);

impl<T> WriterRender<T>
where
  T: StateWriter,
  T::Value: Render + Sized,
{
  pub fn into_widget(self) -> Widget<'static> {
    match self.0.try_into_value() {
      Ok(r) => r.into_widget(),
      Err(this) => match this.into_reader() {
        Ok(r) => ReaderRender(r).into_widget(),
        Err(s) => {
          let modifies = s.raw_modifies();
          let w = ReaderRender(s.clone_reader()).into_widget();
          w.on_build(move |id, ctx| {
            id.dirty_subscribe(modifies, ctx);
          })
        }
      },
    }
  }
}

impl<R> RenderProxy for ReaderRender<R>
where
  R: StateReader,
  R::Value: Render,
{
  type Target<'r> = ReadRef<'r, R::Value>
      where
        Self: 'r;

  #[inline(always)]
  fn proxy(&self) -> Self::Target<'_> { self.0.read() }
}

impl<R: Render> IntoWidgetStrict<'static, RENDER> for State<R> {
  fn into_widget_strict(self) -> Widget<'static> { WriterRender(self).into_widget() }
}

impl<W: Compose + 'static> IntoWidgetStrict<'static, COMPOSE> for State<W> {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> { Compose::compose(self) }
}

impl<T> MultiChild for T
where
  T: StateReader,
  T::Value: MultiChild,
{
}

impl<T> SingleChild for T
where
  T: StateReader,
  T::Value: SingleChild,
{
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;

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
    fn compose(_: impl StateWriter<Value = Self>) -> Widget<'static> { Void.into_widget() }
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
  impl<'c> ComposeChild<'c> for CC {
    type Child = Option<Widget<'c>>;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
      Void.into_widget()
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

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn trait_object_part_data() {
    reset_test_env!();
    let s = State::value(0);
    let m = s.split_writer(|v| PartData::from_ref(v as &mut dyn Any));
    let v: ReadRef<dyn Any> = m.read();
    assert_eq!(*v.downcast_ref::<i32>().unwrap(), 0);

    let s = s.map_writer(|v| PartData::from_ref(v as &mut dyn Any));
    let v: ReadRef<dyn Any> = s.read();
    assert_eq!(*v.downcast_ref::<i32>().unwrap(), 0);
  }
}
