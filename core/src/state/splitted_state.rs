use super::{
  MapReader, ModifyScope, Notifier, ReadRef, StateReader, StateWriter, WriteRef, WriterControl,
};
use crate::{
  context::BuildCtx,
  widget::{Render, RenderBuilder, Widget},
};
use ribir_algo::Sc;
use rxrust::{
  ops::box_it::BoxOp,
  prelude::{BoxIt, ObservableExt},
};
use std::{
  cell::{Cell, RefMut},
  time::Instant,
};

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<O, R, W> {
  origin: O,
  map: R,
  mut_map: W,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  create_at: Instant,
  last_modified: Sc<Cell<Instant>>,
}

pub struct SplittedReader<S, F> {
  origin: S,
  map: F,
  notifier: Notifier,
  create_at: Instant,
  last_modified: Sc<Cell<Instant>>,
}

macro_rules! splitted_reader_impl {
  () => {
    type Value = V;
    type OriginReader = O;
    type Reader = SplittedReader<O::Reader, R>;

    #[track_caller]
    fn read(&self) -> ReadRef<Self::Value> {
      assert!(
        self.create_at > self.origin.time_stamp(),
        "A splitted reader is invalid because its origin state is modified after it created."
      );
      ReadRef::map(self.origin.read(), &self.map)
    }

    #[inline]
    fn clone_reader(&self) -> Self::Reader {
      SplittedReader {
        origin: self.origin.clone_reader(),
        map: self.map.clone(),
        notifier: self.notifier.clone(),
        create_at: self.create_at,
        last_modified: self.last_modified.clone(),
      }
    }

    #[inline]
    fn origin_reader(&self) -> &Self::OriginReader { &self.origin }

    #[inline]
    fn time_stamp(&self) -> Instant { self.last_modified.get() }

    #[inline]
    fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, std::convert::Infallible> {
      let origin = self.origin.clone_reader();
      let create_at = self.create_at;
      self
        .notifier
        .raw_modifies()
        .take_while(move |_| origin.time_stamp() < create_at)
        .box_it()
    }

    #[inline]
    fn try_into_value(self) -> Result<Self::Value, Self> { Err(self) }
  };
}

impl<V, O, R> StateReader for SplittedReader<O, R>
where
  Self: 'static,
  O: StateReader,
  R: Fn(&O::Value) -> &V + Clone,
{
  splitted_reader_impl!();
}

impl<V, O, R, W> StateReader for SplittedWriter<O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  splitted_reader_impl!();
}

impl<V, O, R, W> StateWriter for SplittedWriter<O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  type Writer = SplittedWriter<O::Writer, R, W>;
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
      map: self.map.clone(),
      mut_map: self.mut_map.clone(),
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      last_modified: self.last_modified.clone(),
      create_at: self.create_at,
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, O, R, W> WriterControl for SplittedWriter<O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  #[inline]
  fn last_modified_stamp(&self) -> &Cell<Instant> { &self.last_modified }

  #[inline]
  fn batched_modifies(&self) -> &Cell<ModifyScope> { &self.batched_modify }

  #[inline]
  fn notifier(&self) -> &Notifier { &self.notifier }

  #[inline]
  fn dyn_clone(&self) -> Box<dyn WriterControl> { Box::new(self.clone_writer()) }
}

impl<V, O, R, W> SplittedWriter<O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  pub(super) fn new(origin: O, map: R, mut_map: W) -> Self {
    let create_at = Instant::now();

    Self {
      origin,
      map,
      mut_map,
      notifier: Notifier::default(),
      batched_modify: <_>::default(),
      last_modified: Sc::new(Cell::new(create_at)),
      create_at,
    }
  }

  #[track_caller]
  fn split_ref<'a>(&'a self, mut orig: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    assert!(
      self.create_at > self.origin.time_stamp(),
      "A splitted writer is invalid because its origin state is modified after it created."
    );
    let value = orig
      .value
      .take()
      .map(|orig| RefMut::map(orig, &self.mut_map));

    WriteRef {
      value,
      modified: false,
      modify_scope: orig.modify_scope,
      control: self,
    }
  }
}

impl<V, O, R, W> RenderBuilder for SplittedWriter<O, R, W>
where
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone + 'static,
  W: Fn(&mut O::Value) -> &mut V + Clone,
  V: Render,
{
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    MapReader {
      origin: self.origin.clone_reader(),
      map: self.map.clone(),
    }
    .widget_build(ctx)
  }
}
