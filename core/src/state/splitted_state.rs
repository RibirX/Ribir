use std::cell::Cell;

use ribir_algo::Sc;
use rxrust::{
  ops::box_it::CloneableBoxOp,
  prelude::{BoxIt, ObservableExt},
};

use super::{
  state_cell::PartData, MapWriterAsReader, ModifyScope, Notifier, ReadRef, StateReader,
  StateWatcher, StateWriter, WriteRef, WriterControl,
};
use crate::{
  context::BuildCtx,
  prelude::AppCtx,
  state::state_cell::ValueMutRef,
  ticker::Instant,
  widget::{Render, RenderBuilder, Widget},
};

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<O, W> {
  origin: O,
  splitter: W,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  create_at: Instant,
  last_modified: Sc<Cell<Instant>>,
  ref_count: Sc<Cell<usize>>,
}

impl<O, W> Drop for SplittedWriter<O, W> {
  fn drop(&mut self) {
    if self.ref_count.get() == 1 {
      let mut notifier = self.notifier.clone();
      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      let _ = AppCtx::spawn_local(async move {
        notifier.unsubscribe();
      });
    }
  }
}

impl<V, O, W> StateReader for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  type Value = V;
  type OriginReader = O;
  type Reader = MapWriterAsReader<O::Reader, W>;

  #[track_caller]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.splitter)
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.splitter.clone() }
  }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { &self.origin }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self>
  where
    Self::Value: Sized,
  {
    Err(self)
  }
}

// fixme: we will remove the stamp check after #521 is fixed.
struct StampCheck<R> {
  stamp: Instant,
  reader: R,
}

impl<R: StateReader> CloneableChecker for StampCheck<R> {
  fn check(&self) -> bool { true }
  fn box_clone(&self) -> Box<dyn CloneableChecker> {
    Box::new(StampCheck { stamp: self.stamp, reader: self.reader.clone_reader() })
  }
}

trait CloneableChecker {
  fn check(&self) -> bool;
  fn box_clone(&self) -> Box<dyn CloneableChecker>;
}

impl Clone for Box<dyn CloneableChecker> {
  fn clone(&self) -> Self { self.box_clone() }
}

impl<V, O, W> StateWatcher for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, std::convert::Infallible> {
    let origin = self.origin.clone_reader();
    let create_at = self.create_at;

    // create a cloneable checker to check.
    let checker: Box<dyn CloneableChecker> =
      Box::new(StampCheck { stamp: create_at, reader: origin.clone_reader() });

    self
      .notifier
      .raw_modifies()
      .take_while(move |_| checker.check())
      .box_it()
  }
}

impl<V, O, W> StateWriter for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  type Writer = SplittedWriter<O::Writer, W>;
  type OriginWriter = O;

  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.write()) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.silent()) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.shallow()) }

  fn clone_writer(&self) -> Self::Writer {
    SplittedWriter {
      origin: self.origin.clone_writer(),
      splitter: self.splitter.clone(),
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      last_modified: self.last_modified.clone(),
      create_at: self.create_at,
      ref_count: self.ref_count.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, O, W> WriterControl for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  #[inline]
  fn batched_modifies(&self) -> &Cell<ModifyScope> { &self.batched_modify }

  #[inline]
  fn notifier(&self) -> &Notifier { &self.notifier }

  #[inline]
  fn dyn_clone(&self) -> Box<dyn WriterControl> { Box::new(self.clone_writer()) }
}

impl<V, O, W> SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  pub(super) fn new(origin: O, mut_map: W) -> Self {
    let create_at = Instant::now();

    Self {
      origin,
      splitter: mut_map,
      notifier: Notifier::default(),
      batched_modify: <_>::default(),
      last_modified: Sc::new(Cell::new(create_at)),
      create_at,
      ref_count: Sc::new(Cell::new(1)),
    }
  }

  #[track_caller]
  fn split_ref<'a>(&'a self, mut orig: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    let modify_scope = orig.modify_scope;

    // the origin mark as a silent write, because split writer not effect the origin
    // state in ribir framework level. But keep notify in the data level.
    assert!(!orig.modified);
    orig.modify_scope.remove(ModifyScope::FRAMEWORK);
    orig.modified = true;
    let value =
      ValueMutRef { value: (self.splitter)(&mut orig.value), borrow: orig.value.borrow.clone() };

    WriteRef { value, modified: false, modify_scope, control: self }
  }
}

impl<V, O, W> RenderBuilder for SplittedWriter<O, W>
where
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  fn build(self, ctx: &BuildCtx) -> Widget {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.splitter.clone() }
      .build(ctx)
  }
}
