mod map_state;
mod splitted_state;
mod stateful;

use std::{
  cell::{Cell, Ref, RefCell, RefMut, UnsafeCell},
  convert::Infallible,
  mem::MaybeUninit,
  ops::{Deref, DerefMut},
  time::Instant,
};

use crate::prelude::*;
pub use map_state::*;
use rxrust::ops::box_it::BoxOp;
pub use splitted_state::*;
pub use stateful::*;

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
  /// ## Undefined Behavior
  ///
  /// As `MapReader` is a shortcut to access part of the origin writer, it's
  /// assume its part of data is always valid. If the data is invalid, and you
  /// use `MapReader` to access it, it's undefined behavior.
  #[inline]
  fn map_reader<U, F>(&self, map: F) -> MapReader<Self::Reader, F>
  where
    F: Fn(&Self::Value) -> &U + Clone,
  {
    MapReader { origin: self.clone_reader(), map }
  }
  /// Return the origin reader that this state map or split from . Otherwise
  /// return itself.
  fn origin_reader(&self) -> &Self::OriginReader;

  /// Return the time stamp of the last modifies of this state.
  fn time_stamp(&self) -> Instant;

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
  fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible>;
  /// try convert this state into the value, if there is no other share this
  /// state, otherwise return an error with self.
  fn try_into_value(self) -> Result<Self::Value, Self>
  where
    Self: Sized;
}

pub trait StateWriter: StateReader {
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
  /// Your `mut_map` function will accept a mutable reference, but you should
  /// not modify it, just return a mutable reference to a part of it. Because
  /// Ribir assume you will not modify the origin data in it. If you modify the
  /// origin data in it, no downstream will be notified.
  ///
  /// When you remove/invalid some data across the return writer, you should be
  /// careful. Because that part may be in another state reader or writer, if
  /// you invalid it, other state reader or writer will be invalid too.
  ///
  /// ## Panic
  ///
  /// 1. When the origin writer modifies, the return writer will be invalid,
  ///    access its value will panic.
  /// 2. If modifies the split part data causing any data to be invalid, it may
  ///    panic.

  #[inline]
  fn split_writer<V, R, W>(&self, map: R, mut_map: W) -> SplittedWriter<Self::Writer, R, W>
  where
    R: Fn(&Self::Value) -> &V + Clone + 'static,
    W: Fn(&mut Self::Value) -> &mut V + Clone + 'static,
  {
    SplittedWriter::new(self.clone_writer(), map, mut_map)
  }

  /// Return a new writer by applying a function to the contained value. The
  /// return writer is just a shortcut to access part of the origin writer.
  ///
  /// ## Notice
  ///
  /// Your `mut_map` function will accept a mutable reference, but you should
  /// not modify it, just return a mutable reference to a part of it. Because
  /// Ribir assume you will not modify the origin data in it. If you modify the
  /// origin data in it, no downstream will be notified.
  ///
  /// ## Undefined Behavior
  ///
  /// As `MapWriter` is a shortcut to access part of the origin writer, it's
  /// assume its part of data is always valid. If the data is invalid, and you
  /// use `MapWriter` to access it, it's undefined behavior.
  #[inline]
  fn map_writer<V, R, W>(&self, map: R, mut_map: W) -> MapWriter<Self::Writer, R, W>
  where
    R: Fn(&Self::Value) -> &V + Clone,
    W: Fn(&mut Self::Value) -> &mut V + Clone,
  {
    let origin = self.clone_writer();
    MapWriter { origin, map, mut_map }
  }
}

/// Wraps a borrowed reference to a value in a state.
/// A wrapper type for an immutably borrowed value from a `StateReader`.
pub struct ReadRef<'a, V>(Ref<'a, V>);

pub struct WriteRef<'a, V> {
  value: Option<RefMut<'a, V>>,
  control: &'a dyn WriterControl,
  modify_scope: ModifyScope,
  modified: bool,
}

/// Enum to store both stateless and stateful object.
pub struct State<W>(pub(crate) UnsafeCell<InnerState<W>>);

pub(crate) enum InnerState<W> {
  Data(RefCell<W>),
  Stateful(Stateful<W>),
}

trait WriterControl {
  fn last_modified_stamp(&self) -> &Cell<Instant>;
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
      InnerState::Data(w) => ReadRef::new(w.borrow()),
      InnerState::Stateful(w) => w.read(),
    }
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.as_stateful().clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  #[inline]
  fn time_stamp(&self) -> Instant { self.as_stateful().time_stamp() }

  #[inline]
  fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> {
    self.as_stateful().raw_modifies()
  }

  fn try_into_value(self) -> Result<Self::Value, Self> {
    match self.0.into_inner() {
      InnerState::Data(w) => Ok(w.into_inner()),
      InnerState::Stateful(w) => w.try_into_value().map_err(State::stateful),
    }
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

  pub fn value(value: W) -> Self { State(UnsafeCell::new(InnerState::Data(RefCell::new(value)))) }

  pub fn as_stateful(&self) -> &Stateful<W> {
    match self.inner_ref() {
      InnerState::Data(w) => {
        assert!(w.try_borrow_mut().is_ok());

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
  pub(crate) fn new(r: Ref<'a, V>) -> ReadRef<'a, V> { ReadRef(r) }

  pub(crate) fn map<U>(r: ReadRef<'a, V>, f: impl FnOnce(&V) -> &U) -> ReadRef<'a, U> {
    ReadRef(Ref::map(r.0, f))
  }
}

impl<'a, V> WriteRef<'a, V> {
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
  fn deref(&self) -> &Self::Target {
    // Safety: value always exists except in drop method.
    unsafe { self.value.as_ref().unwrap_unchecked().deref() }
  }
}

impl<'a, W> DerefMut for WriteRef<'a, W> {
  #[track_caller]
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.modified = true;
    self.control.last_modified_stamp().set(Instant::now());
    // Safety: value always exists except in drop method.
    unsafe { self.value.as_mut().unwrap_unchecked().deref_mut() }
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
      AppCtx::spawn_local(async move {
        let scope = control.batched_modifies().replace(ModifyScope::empty());
        control.notifier().next(scope);
      })
      .unwrap();
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
  fn widget_build(self, ctx: &BuildCtx) -> Widget { Compose::compose(self).widget_build(ctx) }
}

impl<P: ComposeChild<Child = Option<C>> + 'static, C> ComposeChildBuilder for State<P> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(self, None).widget_build(ctx)
  }
}

impl<W: SingleChild> SingleChild for State<W> {}
impl<W: MultiChild> MultiChild for State<W> {}

impl<R: Render> RenderBuilder for State<R> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    match self.0.into_inner() {
      InnerState::Data(w) => w.into_inner().widget_build(ctx),
      InnerState::Stateful(w) => w.widget_build(ctx),
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
  T::Value: 'static,
{
  #[inline]
  fn query_inside_first(
    &self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.query_outside_first(type_id, callback)
  }

  fn query_outside_first(
    &self,
    type_id: TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    let any: &T::Value = &self.read();
    if type_id == any.type_id() {
      callback(any)
    } else {
      true
    }
  }
}

macro_rules! impl_compose_builder {
  ($name: ident) => {
    impl<V, W, RM, WM> ComposeBuilder for $name<W, RM, WM>
    where
      W: StateWriter,
      RM: Fn(&W::Value) -> &V + Clone + 'static,
      WM: Fn(&mut W::Value) -> &mut V + Clone + 'static,
      V: Compose + 'static,
    {
      fn widget_build(self, ctx: &crate::context::BuildCtx) -> Widget {
        Compose::compose(self).widget_build(ctx)
      }
    }

    impl<V, W, RM, WM, Child> ComposeChildBuilder for $name<W, RM, WM>
    where
      W: StateWriter,
      RM: Fn(&W::Value) -> &V + Clone + 'static,
      WM: Fn(&mut W::Value) -> &mut V + Clone + 'static,
      V: ComposeChild<Child = Option<Child>> + 'static,
    {
      #[inline]
      fn widget_build(self, ctx: &BuildCtx) -> Widget {
        ComposeChild::compose_child(self, None).widget_build(ctx)
      }
    }
  };
}

impl_compose_builder!(MapWriter);
impl_compose_builder!(SplittedWriter);

#[cfg(test)]
mod tests {
  use std::cell::Cell;

  use ribir_algo::Sc;

  use super::*;
  use crate::{context::AppCtx, reset_test_env, timer::Timer};

  struct Origin {
    a: i32,
    b: i32,
  }

  #[test]
  fn map_same_with_origin() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let map_state = map_writer!($origin.b);

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

  #[test]
  fn split_not_notify_origin() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let split = split_writer!($origin.b);

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
    assert_eq!(
      track_origin.get(),
      ModifyScope::DATA.bits() + ModifyScope::BOTH.bits()
    );
    // splitted downstream will not be notified.
    assert_eq!(track_split.get(), ModifyScope::BOTH.bits());
  }

  #[test]
  #[should_panic]
  fn invalid_split_after_origin_modify() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let split = split_writer!($origin.b);

    origin.write().b = 1;
    // invalid split state
    *split.write() = 1;
  }

  struct C;

  impl Compose for C {
    fn compose(_: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
      fn_widget! { Void }
    }
  }

  #[test]
  fn state_writer_compose_builder() {
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
      let s = Stateful::new((C, 0));
      map_writer!($s.0)
    };
    let _split_writer_compose_widget = fn_widget! {
      let s = Stateful::new((C, 0));
      split_writer!($s.0)
    };
  }

  struct CC;
  impl ComposeChild for CC {
    type Child = Option<Widget>;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
      fn_widget! { @{ Void } }
    }
  }

  #[test]
  fn state_writer_compose_child_builder() {
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
      let s = Stateful::new((CC, 0));
      let w = map_writer!($s.0);
      @$w { @{ Void } }
    };

    let _map_writer_without_child = fn_widget! {
      let s = Stateful::new((CC, 0));
      map_writer!($s.0)
    };

    let _split_writer_with_child = fn_widget! {
      let s = Stateful::new((CC, 0));
      let w = split_writer!($s.0);
      @$w { @{ Void } }
    };

    let _split_writer_without_child = fn_widget! {
      let s = Stateful::new((CC, 0));
      split_writer!($s.0)
    };
  }

  #[test]
  fn state_reader_builder() {
    let _state_render_widget = fn_widget! {
      State::value(Void)
    };

    let _stateful_render_widget = fn_widget! {
      Stateful::new(Void)
    };

    let _reader_render_widget = fn_widget! {
      Stateful::new(Void).clone_reader()
    };

    let _writer_render_widget = fn_widget! {
      Stateful::new(Void).clone_writer()
    };

    let _map_reader_render_widget = fn_widget! {
      Stateful::new((Void, 0)).map_reader(|v| &v.0)
    };

    let _map_writer_render_widget = fn_widget! {
      let s = Stateful::new((Void, 0));
      map_writer!($s.0)
    };

    let _split_writer_render_widget = fn_widget! {
      let s = Stateful::new((Void, 0));
      split_writer!($s.0)
    };
  }
}
