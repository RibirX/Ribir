use super::{MapReader, ModifyScope, Notifier, ReadRef, StateReader, StateWriter, WriteRef};
use crate::{
  context::BuildCtx,
  widget::{Render, RenderBuilder, Widget},
};
use ribir_algo::Sc;
use rxrust::{
  ops::box_it::BoxOp,
  prelude::{ObservableItem, Observer},
  subject::Subject,
  subscription::Subscription,
};
use std::{any::Any, cell::Cell};

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<V, O, R, W>
where
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  origin: O,
  map: R,
  mut_map: W,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  connect_guard: Sc<Box<dyn Any>>,
}

impl<V, O, R, W> StateReader for SplittedWriter<V, O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  type Value = V;
  type OriginReader = O;
  type Reader = MapReader<V, O::Reader, R>;

  fn read(&self) -> ReadRef<Self::Value> { ReadRef::map(self.origin.read(), &self.map) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin: self.origin.clone_reader(),
      map: self.map.clone(),
    }
  }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { &self.origin }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, std::convert::Infallible> {
    self.notifier.modifies()
  }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, std::convert::Infallible> {
    self.notifier.raw_modifies()
  }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self> { Err(self) }
}

impl<V, O, R, W> StateWriter for SplittedWriter<V, O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  type Writer = SplittedWriter<V, O::Writer, R, W>;
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
      connect_guard: self.connect_guard.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, O, R, W> SplittedWriter<V, O, R, W>
where
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  pub(super) fn new(origin: O, map: R, mut_map: W) -> Self {
    let notifier = Notifier::default();
    let c_modifier = notifier.clone();

    let h = origin
      .raw_modifies()
      .subscribe(move |v| c_modifier.raw_modifies().next(v))
      .unsubscribe_when_dropped();

    Self {
      origin,
      map,
      mut_map,
      notifier,
      batched_modify: <_>::default(),
      connect_guard: Sc::new(Box::new(h)),
    }
  }

  fn split_ref<'a>(&'a self, origin_ref: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    WriteRef::split_map(
      origin_ref,
      &self.mut_map,
      self.batched_modify.clone(),
      self.notifier.clone(),
    )
  }
}

impl<V, O, R, W> RenderBuilder for SplittedWriter<V, O, R, W>
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
