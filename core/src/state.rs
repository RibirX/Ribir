mod map_state;
mod prior_op;
mod stateful;
mod watcher;
use std::{cell::UnsafeCell, convert::Infallible, mem::MaybeUninit, ops::DerefMut};
pub mod state_cell;

pub use map_state::*;
pub use prior_op::*;
use rxrust::ops::box_it::{BoxOp, CloneableBoxOp};
pub use state_cell::*;
pub use stateful::*;
pub use watcher::*;

use crate::prelude::*;

pub type PartialId = CowArc<str>;
pub type PartialPath = Vec<PartialId>;

/// The `StateReader` trait allows for reading, clone and map the state.
pub trait StateReader: 'static {
  /// The value type of this state.
  type Value: ?Sized;
  type Reader: StateReader<Value = Self::Value>
  where
    Self: Sized;

  /// Return a reference of this state.
  fn read(&self) -> ReadRef<Self::Value>;

  /// Return a boxed reader of this state.
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>>;

  /// Return a cloned reader of this state.
  fn clone_reader(&self) -> Self::Reader
  where
    Self: Sized;
  /// Maps an reader to another by applying a function to a contained
  /// value. The return reader is just a shortcut to access part of the origin
  /// reader.
  ///
  /// Note, `MapReader` is a shortcut to access a portion of the original
  /// reader. It's assumed that the `map` function returns a part of the
  /// original data, not a cloned portion. Otherwise, the returned reader will
  /// not respond to state changes.
  #[inline]
  fn part_reader<U: ?Sized, F>(&self, map: F) -> PartReader<Self::Reader, F>
  where
    F: Fn(&Self::Value) -> PartRef<U> + Clone,
    Self: Sized,
  {
    PartReader { origin: self.clone_reader(), part_map: map }
  }

  /// try convert this state into the value, if there is no other share this
  /// state, otherwise return an error with self.
  fn try_into_value(self) -> Result<Self::Value, Self>
  where
    Self: Sized,
    Self::Value: Sized,
  {
    Err(self)
  }
}

pub trait StateWatcher: StateReader {
  type Watcher: StateWatcher<Value = Self::Value>
  where
    Self: Sized;

  /// Convert the writer to a reader if no other writers exist.
  fn into_reader(self) -> Result<Self::Reader, Self>
  where
    Self: Sized;

  /// Return a modifies `Rx` stream of the state, user can subscribe it to
  /// response the state changes.
  fn modifies(&self) -> BoxOp<'static, ModifyInfo, Infallible> {
    self
      .raw_modifies()
      .filter(|s| s.contains(ModifyScope::DATA))
      .box_it()
  }

  /// Return a modifies `Rx` stream of the state, including all modifies. Use
  /// `modifies` instead if you only want to response the data changes.
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible>;

  /// Clone a boxed watcher that can be used to observe the modifies of the
  /// state.
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>>;

  /// Clone a watcher that can be used to observe the modifies of the state.
  fn clone_watcher(&self) -> Self::Watcher
  where
    Self: Sized;

  /// Return a new watcher by applying a function to the contained value.
  fn part_watcher<U: ?Sized, F>(&self, map: F) -> Watcher<PartReader<Self::Reader, F>>
  where
    F: Fn(&Self::Value) -> PartRef<U> + Clone,
    Self: Sized,
  {
    let reader = self.part_reader(map);
    Watcher::new(reader, self.raw_modifies())
  }
}

pub trait StateWriter: StateWatcher {
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

  /// Clone a boxed writer of this state.
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>>;

  /// Clone a writer of this state.
  fn clone_writer(&self) -> Self
  where
    Self: Sized;

  /// part_writer
  ///
  /// Creates a new writer that derives a subset or transformation of the
  /// original writer's value. The returned writer shares the same underlying
  /// data with the original writer.
  /// - If ID is None, the created writer is just a partial writer of the
  ///   original and sharing the same notification.
  /// - If ID is specified, the created writer's notification mechanisms are
  ///   adjusted as follows:
  /// 1. **Unidirectional Propagation**: Modifications made through the split
  ///    writer will trigger updates on the original writer, but changes from
  ///    the original writer will not propagate to the split writer.
  ///
  /// 2. **ID-Based Update Sharing**: Multiple split writers created with the
  ///    same ID will share update notifications. This means modifications in
  ///    any split writer with the same ID will be treated as a single source of
  ///    truth for the original writer's dependencies.
  ///
  /// 3. **Isolated Update Domains**: Split writers created with different IDs
  ///    operate in isolated notification domains. Updates from one ID will not
  ///    trigger notifications in split writers with different IDs, even if they
  ///    share the same original writer.
  ///
  /// If you need a writer that mirrors the original writer's notification
  /// behavior exactly, use `part_writer(...)` instead.
  ///
  /// ##Notice
  ///
  /// The `mut_map` function accepts a mutable reference, but it should not be
  /// modified. Instead, return a mutable reference to a part of it. Ribir
  /// uses this to access a portion of the original data, so a read operation
  /// may also call it. Ribir assumes that the original data will not be
  /// modified within this function. Therefore, if the original data is
  /// modified, no downstream will be notified.
  ///
  /// ## Example, Use part_writer to map to part of the state
  ///
  /// ```rust no_run
  /// use ribir::prelude::*;
  ///
  /// struct MyApp {
  ///   name: String,
  ///   // ...
  /// }
  ///
  /// fn title_widget(app: impl StateWriter<Value = String>) -> Widget<'static> {
  ///   fn_widget! { @Text { text: pipe!($app.write().clone()) } }.into_widget()
  /// }
  ///
  /// let w = fn_widget! {
  ///   let app = Stateful::new(MyApp { name: String::new() /* ... */ });
  ///   title_widget(app.part_writer(None, |app| PartMut::new(&mut app.name)))
  /// };
  /// App::run(w);
  /// ```
  ///
  /// ## Example, Use part_writer with a unique identifier to create isolated Writer.
  /// part_writer with a unique identifier to bind to each item in the list,
  /// enabling isolated modification of individual items
  /// ```rust no_run
  /// use ribir::prelude::*;
  ///
  /// let w = fn_widget! {
  ///    let  items = Stateful::new(vec![String::new(), String::new()]);
  ///    @Column {
  ///      @FilledButton {
  ///        on_tap: move |_| $items.write().push(String::new()),
  ///        @ { "Add Item" }
  ///      }
  ///      @ {
  ///        pipe!($items.len())
  ///          .value_chain(|f| f.filter(|(info, _)| info.partial_path().is_none()).box_it())
  ///          .map(move|cnt| {
  ///            (0..cnt).map(move |i| fn_widget!{
  ///              let _hint = || $items.write();
  ///              let item = items.part_writer(
  ///                 Some(&format!("item {i}")),
  ///                 move |items| PartMut::new(items.get_mut(i).unwrap())
  ///              );
  ///              let input = @Input {};
  ///              $input.write().set_text(&$item);
  ///              watch!($input.text().clone())
  ///                .subscribe(move |text| {
  ///                  *$item.write() = text.to_string()
  ///                });
  ///              input
  ///            })
  ///        })
  ///      }
  ///    }
  /// };
  ///
  /// App::run(w);
  /// ```
  fn part_writer<V: ?Sized + 'static, M>(
    &self, id: Option<&str>, part_map: M,
  ) -> PartWriter<Self, M>
  where
    M: Fn(&mut Self::Value) -> PartMut<V> + Clone + 'static,
    Self: Sized,
  {
    PartWriter { origin: self.clone_writer(), part_map, id: id.map(|id| id.to_string().into()) }
  }
}

pub struct WriteRef<'a, V: ?Sized> {
  value: ValueMutRef<'a, V>,
  info: &'a Sc<WriterInfo>,
  modify_scope: ModifyScope,
  modified: bool,
  partial: Option<Vec<&'a PartialId>>,
}

/// Enum to store both stateless and stateful object.
pub struct State<W>(pub(crate) UnsafeCell<InnerState<W>>);

pub(crate) enum InnerState<W> {
  Data(StateCell<W>),
  Stateful(Stateful<W>),
}

impl<T: 'static> StateReader for State<T> {
  type Value = T;
  type Reader = Reader<T>;

  fn read(&self) -> ReadRef<T> {
    match self.inner_ref() {
      InnerState::Data(w) => w.read(),
      InnerState::Stateful(w) => w.read(),
    }
  }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.as_stateful().clone_reader() }

  fn try_into_value(self) -> Result<Self::Value, Self> {
    match self.0.into_inner() {
      InnerState::Data(w) => Ok(w.into_inner()),
      InnerState::Stateful(w) => w.try_into_value().map_err(State::stateful),
    }
  }
}

impl<T: 'static> StateWatcher for State<T> {
  type Watcher = Watcher<Self::Reader>;

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
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    self.as_stateful().raw_modifies()
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<T: 'static> StateWriter for State<T> {
  #[inline]
  fn write(&self) -> WriteRef<T> { self.as_stateful().write() }

  #[inline]
  fn silent(&self) -> WriteRef<T> { self.as_stateful().silent() }

  #[inline]
  fn shallow(&self) -> WriteRef<T> { self.as_stateful().shallow() }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  #[inline]
  fn clone_writer(&self) -> Self { State::stateful(self.as_stateful().clone_writer()) }
}

impl<W> State<W> {
  pub fn stateful(stateful: Stateful<W>) -> Self {
    State(UnsafeCell::new(InnerState::Stateful(stateful)))
  }

  pub fn value(value: W) -> Self { State(UnsafeCell::new(InnerState::Data(StateCell::new(value)))) }

  pub fn into_stateful(self) -> Stateful<W>
  where
    W: 'static,
  {
    self.as_stateful().clone_writer()
  }
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
  pub fn map<U: ?Sized, M>(
    mut orig: WriteRef<'a, V>, part_map: M, partial: Option<&'a PartialId>,
  ) -> WriteRef<'a, U>
  where
    M: Fn(&mut V) -> PartMut<U>,
  {
    let inner = part_map(&mut orig.value).inner;
    let borrow = orig.value.borrow.clone();
    let value = ValueMutRef { inner, borrow };

    let partial = if let Some(partial) = partial {
      let mut p = orig.partial.clone().unwrap_or_default();
      p.push(partial);
      Some(p)
    } else {
      orig.partial.clone()
    };
    WriteRef { value, modified: false, modify_scope: orig.modify_scope, info: orig.info, partial }
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
  ///   WriteRef::filter_map(b1, |v| v.get_mut(1).map(PartMut::<u32>::new), None);
  /// assert_eq!(*b2.unwrap(), 2);
  /// ```
  pub fn filter_map<U: ?Sized, M>(
    mut orig: WriteRef<'a, V>, part_map: M, partial: Option<&'a PartialId>,
  ) -> Result<WriteRef<'a, U>, Self>
  where
    M: Fn(&mut V) -> Option<PartMut<U>>,
  {
    match part_map(&mut orig.value) {
      Some(inner) => {
        let inner = inner.inner;
        let borrow = orig.value.borrow.clone();
        let value = ValueMutRef { inner, borrow };

        let partial = if let Some(partial) = partial {
          let mut p = orig.partial.clone().unwrap_or_default();
          p.push(partial);
          Some(p)
        } else {
          orig.partial.clone()
        };
        Ok(WriteRef {
          value,
          modified: false,
          modify_scope: orig.modify_scope,
          info: orig.info,
          partial,
        })
      }
      None => Err(orig),
    }
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
      let partial = self.partial.as_ref().map(|v| {
        v.iter()
          .map(|i| (**i).clone())
          .collect::<Vec<_>>()
      });
      let _ = AppCtx::spawn_local(async move {
        let scope = info
          .batched_modifies
          .replace(ModifyScope::empty());
        info.notifier.next(ModifyInfo { scope, partial });
      });
    } else {
      batched_modifies.set(*modify_scope | batched_modifies.get());
    }
  }
}

impl<V: ?Sized + 'static> StateReader for Box<dyn StateReader<Value = V>> {
  type Value = V;
  type Reader = Self;

  #[inline]
  fn read(&self) -> ReadRef<'_, V> { (**self).read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    (**self).clone_boxed_reader()
  }

  fn clone_reader(&self) -> Self::Reader { self.clone_boxed_reader() }
}

impl<V: ?Sized + 'static> StateReader for Box<dyn StateWatcher<Value = V>> {
  type Value = V;
  type Reader = Box<dyn StateReader<Value = V>>;

  #[inline]
  fn read(&self) -> ReadRef<'_, V> { (**self).read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    (**self).clone_boxed_reader()
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.clone_boxed_reader() }
}

impl<V: ?Sized + 'static> StateWatcher for Box<dyn StateWatcher<Value = V>> {
  type Watcher = Box<dyn StateWatcher<Value = V>>;

  #[inline]
  fn into_reader(self) -> Result<Self::Reader, Self> { Err(self) }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    (**self).raw_modifies()
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    (**self).clone_boxed_watcher()
  }

  #[inline]
  fn clone_watcher(&self) -> Self::Watcher { self.clone_boxed_watcher() }
}

impl<V: ?Sized + 'static> StateReader for Box<dyn StateWriter<Value = V>> {
  type Value = V;
  type Reader = Box<dyn StateReader<Value = V>>;

  #[inline]
  fn read(&self) -> ReadRef<'_, V> { (**self).read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    (**self).clone_boxed_reader()
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.clone_boxed_reader() }
}

impl<V: ?Sized + 'static> StateWatcher for Box<dyn StateWriter<Value = V>> {
  type Watcher = Box<dyn StateWatcher<Value = Self::Value>>;

  #[inline]
  fn into_reader(self) -> Result<Self::Reader, Self> { Err(self) }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    (**self).raw_modifies()
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    (**self).clone_boxed_watcher()
  }

  #[inline]
  fn clone_watcher(&self) -> Self::Watcher { self.clone_boxed_watcher() }
}

impl<V: ?Sized + 'static> StateWriter for Box<dyn StateWriter<Value = V>> {
  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { (**self).write() }
  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { (**self).silent() }
  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { (**self).shallow() }
  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    (**self).clone_boxed_writer()
  }
  #[inline]
  fn clone_writer(&self) -> Self { self.clone_boxed_writer() }
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
    let map_state = origin.part_writer(None, |v| PartMut::new(&mut v.b));

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
  fn split_notify() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let split_a = origin.part_writer(Some("a"), |v| PartMut::new(&mut v.a));
    let split_b = origin.part_writer(Some("b"), |v| PartMut::new(&mut v.b));

    let track_origin = Sc::new(Cell::new(0));
    let track_split_a = Sc::new(Cell::new(0));
    let track_split_b = Sc::new(Cell::new(0));

    let c_origin = track_origin.clone();
    origin.modifies().subscribe(move |s| {
      c_origin.set(c_origin.get() + s.scope().bits());
    });

    let c_split_a = track_split_a.clone();
    split_a.modifies().subscribe(move |s| {
      c_split_a.set(c_split_a.get() + s.scope().bits());
    });

    let c_split_b = track_split_b.clone();
    split_b.modifies().subscribe(move |s| {
      c_split_b.set(c_split_b.get() + s.scope().bits());
    });

    *split_a.write() = 0;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), ModifyScope::BOTH.bits());
    assert_eq!(track_split_a.get(), ModifyScope::BOTH.bits());
    assert_eq!(track_split_b.get(), 0);

    track_origin.set(0);
    track_split_a.set(0);

    *split_b.write() = 0;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(track_origin.get(), ModifyScope::BOTH.bits());
    assert_eq!(track_split_b.get(), ModifyScope::BOTH.bits());
    assert_eq!(track_split_a.get(), 0);

    track_origin.set(0);
    track_split_b.set(0);

    origin.write().a = 0;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();
    assert_eq!(track_origin.get(), ModifyScope::BOTH.bits());
    assert_eq!(track_split_b.get(), 0);
    assert_eq!(track_split_a.get(), 0);
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

    let _part_writer_compose_widget = fn_widget! {
      Stateful::new((C, 0))
        .part_writer(None, |v| PartMut::new(&mut v.0))
    };
    let _part_writer_compose_widget = fn_widget! {
      Stateful::new((C, 0))
        .part_writer(Some("C"), |v| PartMut::new(&mut v.0))
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

    let _part_writer_with_child = fn_widget! {
      let w = Stateful::new((CC, 0))
        .part_writer(None, |v| PartMut::new(&mut v.0));
      @$w { @{ Void } }
    };

    let _part_writer_without_child = fn_widget! {
      Stateful::new((CC, 0))
        .part_writer(None, |v| PartMut::new(&mut v.0))
    };

    let _part_writer_with_child = fn_widget! {
      let w = Stateful::new((CC, 0))
        .part_writer(Some(""), |v| PartMut::new(&mut v.0));
      @$w { @{ Void } }
    };

    let _part_writer_without_child = fn_widget! {
      Stateful::new((CC, 0))
        .part_writer(Some(""), |v| PartMut::new(&mut v.0))
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

    let _part_reader_render_widget = fn_widget! {
      Stateful::new((Void, 0)).part_reader(|v| PartRef::new(&v.0))
    };

    let _part_writer_render_widget = fn_widget! {
      Stateful::new((Void, 0))
        .part_writer(None, |v| PartMut::new(&mut v.0))
    };

    let _part_writer_render_widget = fn_widget! {
      Stateful::new((Void, 0))
        .part_writer(Some(""), |v| PartMut::new(&mut v.0))
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn trait_object_part_data() {
    reset_test_env!();
    let s = State::value(0);
    let m = s.part_writer(Some("0"), |v| PartMut::new(v as &mut dyn Any));
    let v: ReadRef<dyn Any> = m.read();
    assert_eq!(*v.downcast_ref::<i32>().unwrap(), 0);

    let s = s.part_writer(None, |v| PartMut::new(v as &mut dyn Any));
    let v: ReadRef<dyn Any> = s.read();
    assert_eq!(*v.downcast_ref::<i32>().unwrap(), 0);
  }
}
