use std::convert::Infallible;

use rxrust::ops::box_it::CloneableBoxOp;

use super::{
  state_cell::PartData, ModifyScope, ReadRef, StateReader, StateWatcher, StateWriter, WriteRef,
};
use crate::{
  context::BuildCtx,
  render_helper::{RenderProxy, RenderTarget},
  widget::{Render, RenderBuilder, Widget},
};

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader.
pub struct MapReader<S, F> {
  pub(super) origin: S,
  pub(super) part_map: F,
}

pub struct MapWriter<W, WM> {
  pub(super) origin: W,
  pub(super) part_map: WM,
}

pub struct MapWriterAsReader<W, M> {
  pub(super) origin: W,
  pub(super) part_map: M,
}

impl<S, V, M> StateReader for MapReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: Fn(&S::Value) -> PartData<V> + Clone + 'static,
{
  type Value = V;
  type OriginReader = S;
  type Reader = MapReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { ReadRef::map(self.origin.read(), &self.part_map) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
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

impl<S, V, M> StateReader for MapWriterAsReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: Fn(&mut S::Value) -> PartData<V> + Clone,
{
  type Value = V;
  type OriginReader = S;
  type Reader = MapWriterAsReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.part_map)
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
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

impl<V, S, M> StateReader for MapWriter<S, M>
where
  Self: 'static,
  S: StateWriter,
  M: Fn(&mut S::Value) -> PartData<V> + Clone,
{
  type Value = V;
  type OriginReader = S;
  type Reader = MapWriterAsReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.part_map)
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
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

impl<V, W, M> StateWatcher for MapWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartData<V> + Clone,
{
  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.origin.raw_modifies()
  }
}

impl<V, W, M> StateWriter for MapWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartData<V> + Clone,
{
  type Writer = MapWriter<W::Writer, M>;
  type OriginWriter = W;

  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.write(), &self.part_map) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.silent(), &self.part_map) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> {
    WriteRef::map(self.origin.shallow(), &self.part_map)
  }

  #[inline]
  fn clone_writer(&self) -> Self::Writer {
    MapWriter { origin: self.origin.clone_writer(), part_map: self.part_map.clone() }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, S, F> RenderTarget for MapReader<S, F>
where
  S: StateReader,
  F: Fn(&S::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  type Target = V;

  fn proxy<T>(&self, f: impl FnOnce(&Self::Target) -> T) -> T {
    let v = self.read();
    f(&*v)
  }
}

impl<V, S, F> RenderTarget for MapWriterAsReader<S, F>
where
  S: StateReader,
  F: Fn(&mut S::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  type Target = V;

  fn proxy<T>(&self, f: impl FnOnce(&Self::Target) -> T) -> T {
    let v = self.read();
    f(&*v)
  }
}

impl<V, S, F> RenderBuilder for MapReader<S, F>
where
  S: StateReader,
  F: Fn(&S::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { RenderProxy::new(self).build(ctx) }
}

impl<V, S, F> RenderBuilder for MapWriterAsReader<S, F>
where
  S: StateReader,
  F: Fn(&mut S::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { RenderProxy::new(self).build(ctx) }
}

impl<V, S, WM> RenderBuilder for MapWriter<S, WM>
where
  Self: 'static,
  S: StateWriter,
  WM: Fn(&mut S::Value) -> PartData<V> + Clone,
  V: Render,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { self.clone_reader().build(ctx) }
}
