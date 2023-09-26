mod map_state;
mod splitted_state;
mod stateful;

use std::{
  cell::UnsafeCell,
  convert::Infallible,
  mem::MaybeUninit,
  ops::{Deref, DerefMut},
};

use crate::prelude::*;
pub use map_state::*;
use rxrust::{ops::box_it::BoxOp, subject::Subject};
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
  /// The reference type that can read the value of the state.
  type Ref<'a>: Deref<Target = Self::Value>;
  /// Return a reference of this state.
  fn read(&'_ self) -> Self::Ref<'_>;
  /// get a clone of this state that only can read.
  fn clone_reader(&self) -> Self::Reader;
  /// Maps an reader to another by applying a function to a contained
  /// value. The return reader and the origin reader are the same reader. So
  /// when one of them is modified, they will both be notified.
  fn map_reader<F, Target>(&self, f: F) -> MapReader<Target, Self::Reader, F>
  where
    F: FnOnce(&Self::Value) -> &Target + Copy,
  {
    MapReader::new(self.clone_reader(), f)
  }
  /// Return the origin reader that this state map or split from . Otherwise
  /// return itself.
  fn origin_reader(&self) -> &Self::OriginReader;
  /// Return a modifies `Rx` stream of the state, user can subscribe it to
  /// response the state changes.
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible>;
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible>;
}

pub trait StateWriter: StateReader {
  type Writer: StateWriter<Value = Self::Value>;
  type OriginWriter: StateWriter;
  type RefWrite<'a>: RefWrite<Target = Self::Value>
  where
    Self: 'a;

  /// Return a write reference of this state.
  fn write(&'_ self) -> Self::RefWrite<'_>;
  /// Return a silent write reference which notifies will be ignored by the
  /// framework.
  fn silent(&'_ self) -> Self::RefWrite<'_>;
  /// Convert this state reference to a shallow reference. Modify across this
  /// reference will notify framework only. That means the modifies on shallow
  /// reference should only effect framework but not effect on data. eg.
  /// temporary to modify the state and then modifies it back to trigger the
  /// view update. Use it only if you know how a shallow reference works.
  fn shallow(&'_ self) -> Self::RefWrite<'_>;
  /// Clone this state writer.
  fn clone_writer(&self) -> Self::Writer;
  /// Return the origin writer that this state map or split from.
  fn origin_writer(&self) -> &Self::OriginWriter;
  /// Return a new writer by applying a function to the contained value.
  ///
  /// This writer share the same state with the origin writer. But has it's own
  /// notifier. When modifies across the return writer, the downstream
  /// subscribed on the origin state will not be notified. But when modifies
  /// across the origin writer, the downstream subscribed on the return writer
  /// will be notified.
  ///
  /// If you want split a new writer that has same behavior with the origin
  /// writer, you can use `map_reader(..).into_writer(..)`.
  fn split_writer<Target, R, W>(
    &self,
    read_map: R,
    writer_map: W,
  ) -> SplittedWriter<Target, Self::Writer, R, W>
  where
    R: FnOnce(&Self::Value) -> &Target + Copy,
    W: FnOnce(&mut Self::Value) -> &mut Target + Copy,
  {
    SplittedWriter::new(self.clone_writer(), read_map, writer_map)
  }

  fn map_writer<Target, R, W>(
    &self,
    read_map: R,
    writer_map: W,
  ) -> MapWriter<Target, Self::Writer, R, W>
  where
    R: FnOnce(&Self::Value) -> &Target + Copy,
    W: FnOnce(&mut Self::Value) -> &mut Target + Copy,
  {
    MapWriter::new(self.clone_writer(), read_map, writer_map)
  }
}

pub trait RefWrite: DerefMut {
  /// Forget all modifies of this reference. So all the modifies occurred on
  /// this reference before this call will not be notified. Return true if there
  /// is any modifies on this reference.
  fn forget_modifies(&mut self) -> bool;
}

/// Enum to store both stateless and stateful object.
pub struct State<W>(pub(crate) UnsafeCell<InnerState<W>>);

pub(crate) enum InnerState<W> {
  Data(StateData<W>),
  Stateful(Stateful<W>),
}

impl<T: 'static> StateReader for State<T> {
  type Value = T;
  type OriginReader = Self;
  type Reader = Reader<T>;
  type Ref<'a> = ReadRef<'a, T>;
  fn read(&'_ self) -> Self::Ref<'_> {
    match self.inner_ref() {
      InnerState::Data(w) => w.read(),
      InnerState::Stateful(w) => w.read(),
    }
  }

  fn clone_reader(&self) -> Self::Reader { self.as_stateful().clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self }

  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.as_stateful().modifies() }

  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.as_stateful().raw_modifies()
  }
}

impl<T: 'static> StateWriter for State<T> {
  type Writer = Writer<T>;
  type OriginWriter = Self;
  type RefWrite<'a> = WriteRef<'a,T>
  where
    Self: 'a;

  #[inline]
  fn write(&'_ self) -> Self::RefWrite<'_> { self.as_stateful().write() }

  #[inline]
  fn silent(&'_ self) -> Self::RefWrite<'_> { self.as_stateful().silent() }

  #[inline]
  fn shallow(&'_ self) -> Self::RefWrite<'_> { self.as_stateful().shallow() }

  #[inline]
  fn clone_writer(&self) -> Self::Writer { self.as_stateful().clone_writer() }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { self }
}

impl<W> State<W> {
  pub fn stateful(stateful: Stateful<W>) -> Self {
    State(UnsafeCell::new(InnerState::Stateful(stateful)))
  }

  pub fn value(value: W) -> Self { State(UnsafeCell::new(InnerState::Data(StateData::new(value)))) }

  /// Unwrap the state and return the inner value.
  ///
  /// # Panics
  /// panic if the state is stateful or already attached data.
  pub fn into_value(self) -> W {
    match self.0.into_inner() {
      InnerState::Data(w) => w.into_inner(),
      InnerState::Stateful(w) => w
        .try_into_inner()
        .unwrap_or_else(|_| panic!("can't convert stateful to value")),
    }
  }

  pub fn as_stateful(&self) -> &Stateful<W> {
    match self.inner_ref() {
      InnerState::Data(w) => {
        w.assert_is_not_used();

        let mut uninit: MaybeUninit<_> = MaybeUninit::uninit();
        // Safety: we already check there is no other reference to the state data.
        unsafe {
          std::ptr::copy(w, uninit.as_mut_ptr(), 1);
          let stateful = InnerState::Stateful(Stateful::from_state_data(uninit.assume_init()));
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

impl<C: Compose> ComposeBuilder for State<C> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { Compose::compose(self).widget_build(ctx) }
}

impl<P: ComposeChild<Child = Option<C>>, C> ComposeChildBuilder for State<P> {
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
    let v = self.read();
    let any = &*v;
    any.type_id() == type_id && callback(any)
  }
}

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
  fn path_state_router_test() {
    reset_test_env!();

    let origin = State::value(Origin { a: 0, b: 0 });
    let a = split_writer!($origin.a);
    let b = map_writer!($origin.b);

    let track_origin = Sc::new(Cell::new(0));
    let track_a = Sc::new(Cell::new(0));
    let track_b = Sc::new(Cell::new(0));

    let c_origin = track_origin.clone();
    origin.modifies().subscribe(move |_| {
      c_origin.set(c_origin.get() + 1);
    });

    let c_a = track_a.clone();
    a.modifies().subscribe(move |_| {
      c_a.set(c_a.get() + 1);
    });

    let c_b = track_b.clone();
    b.modifies().subscribe(move |_| {
      c_b.set(c_b.get() + 1);
    });

    origin.write().a = 1;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), 1);
    assert_eq!(track_a.get(), 1);
    assert_eq!(track_b.get(), 1);

    *a.write() = 1;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), 1);
    assert_eq!(track_a.get(), 2);
    assert_eq!(track_b.get(), 1);

    *b.write() = 1;
    Timer::wake_timeout_futures();
    AppCtx::run_until_stalled();

    assert_eq!(track_origin.get(), 2);
    assert_eq!(track_a.get(), 3);
    assert_eq!(track_b.get(), 2);
  }
}
