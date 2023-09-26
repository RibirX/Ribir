use crate::{
  context::BuildCtx,
  render_helper::{RenderProxy, RenderTarget},
  widget::{Render, RenderBuilder, Widget},
};

use super::{ModifyScope, RefWrite, StateReader, StateWriter};
use rxrust::{ops::box_it::BoxOp, subject::Subject};
use std::{
  convert::Infallible,
  ops::{Deref, DerefMut},
};

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader, It's also have
/// the same modifier with the origin state.
// Keep the `V` as the first generic, so the user know the actual value type
// when ide hints.
pub struct MapReader<V, R: StateReader, F: FnOnce(&R::Value) -> &V + Copy> {
  origin_reader: R,
  map_fn: F,
}

pub struct MapWriter<
  V,
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy,
  WM: FnOnce(&mut W::Value) -> &mut V + Copy,
> {
  origin_writer: W,
  map_reader: RM,
  map_writer: WM,
}

/// The read reference of `MapReader`.
pub struct MapReadRef<V, O: Deref, F: FnOnce(&O::Target) -> &V + Copy> {
  origin_ref: O,
  map_fn: F,
}

pub struct MapWriteRef<V, O, R, W>
where
  O: DerefMut,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  origin_ref: O,
  map_read: R,
  map_write: W,
}

impl<V, R, F> MapReader<V, R, F>
where
  R: StateReader,
  F: FnOnce(&R::Value) -> &V + Copy,
{
  #[inline]
  pub fn new(origin_reader: R, map_fn: F) -> Self { Self { origin_reader, map_fn } }
}

impl<V, W, RM> MapReader<V, W, RM>
where
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy,
{
  /// Convert a `MapRender` to a `MapWriter` by add a write map function.
  #[inline]
  pub fn into_writer<WM: FnOnce(&mut W::Value) -> &mut V + Copy>(
    self,
    map_fn: WM,
  ) -> MapWriter<V, W, RM, WM> {
    MapWriter {
      map_reader: self.map_fn,
      map_writer: map_fn,
      origin_writer: self.origin_reader,
    }
  }
}

impl<V, W, RM, WM> MapWriter<V, W, RM, WM>
where
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy,
  WM: FnOnce(&mut W::Value) -> &mut V + Copy,
{
  #[inline]
  pub fn new(origin_state: W, map_reader: RM, map_writer: WM) -> Self {
    Self {
      origin_writer: origin_state,
      map_reader,
      map_writer,
    }
  }
}

impl<V, O: Deref, F: FnOnce(&O::Target) -> &V + Copy> MapReadRef<V, O, F> {
  #[inline]
  pub fn new(origin_ref: O, map_fn: F) -> Self { MapReadRef { origin_ref, map_fn } }
}

impl<V, R, M> StateReader for MapReader<V, R, M>
where
  Self: 'static,
  R: StateReader,
  M: FnOnce(&R::Value) -> &V + Copy,
{
  type Value = V;
  type OriginReader = R;
  type Reader = MapReader<V, R::Reader, M>;
  type Ref<'a> = MapReadRef<V, R::Ref<'a>, M>;

  #[inline]
  fn read(&'_ self) -> Self::Ref<'_> {
    MapReadRef {
      origin_ref: self.origin_reader.read(),
      map_fn: self.map_fn,
    }
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    let origin_state = self.origin_reader.clone_reader();
    MapReader {
      origin_reader: origin_state,
      map_fn: self.map_fn,
    }
  }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { &self.origin_reader }
  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.origin_reader.modifies() }
  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.origin_reader.raw_modifies()
  }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self> { Err(self) }
}

impl<V, W, RM, WM> StateReader for MapWriter<V, W, RM, WM>
where
  Self: 'static,
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy,
  WM: FnOnce(&mut W::Value) -> &mut V + Copy,
{
  type Value = V;
  type OriginReader = W;
  type Reader = MapReader<V, W::Reader, RM>;
  type Ref<'a> = MapReadRef<V, W::Ref<'a>, RM>;

  #[inline]
  fn read(&'_ self) -> Self::Ref<'_> {
    MapReadRef {
      origin_ref: self.origin_writer.read(),
      map_fn: self.map_reader,
    }
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin_reader: self.origin_writer.clone_reader(),
      map_fn: self.map_reader,
    }
  }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { &self.origin_writer }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.origin_writer.modifies() }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
    self.origin_writer.raw_modifies()
  }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self> { Err(self) }
}

impl<V, W, RM, WM> StateWriter for MapWriter<V, W, RM, WM>
where
  Self: 'static,
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy,
  WM: FnOnce(&mut W::Value) -> &mut V + Copy,
{
  type Writer = MapWriter<V, W::Writer, RM, WM>;
  type OriginWriter = W;
  type RefWrite<'a> = MapWriteRef<V, W::RefWrite<'a>, RM, WM>;

  fn write(&'_ self) -> Self::RefWrite<'_> {
    MapWriteRef {
      origin_ref: self.origin_writer.write(),
      map_read: self.map_reader,
      map_write: self.map_writer,
    }
  }

  #[inline]
  fn silent(&'_ self) -> Self::RefWrite<'_> {
    MapWriteRef {
      origin_ref: self.origin_writer.silent(),
      map_read: self.map_reader,
      map_write: self.map_writer,
    }
  }

  #[inline]
  fn shallow(&'_ self) -> Self::RefWrite<'_> {
    MapWriteRef {
      origin_ref: self.origin_writer.shallow(),
      map_read: self.map_reader,
      map_write: self.map_writer,
    }
  }

  #[inline]
  fn clone_writer(&self) -> Self::Writer {
    MapWriter {
      origin_writer: self.origin_writer.clone_writer(),
      map_reader: self.map_reader,
      map_writer: self.map_writer,
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin_writer }
}

impl<V, O, F> std::ops::Deref for MapReadRef<V, O, F>
where
  O: Deref,
  F: FnOnce(&O::Target) -> &V + Copy,
{
  type Target = V;
  #[inline]
  fn deref(&self) -> &Self::Target { (self.map_fn)(self.origin_ref.deref()) }
}

impl<V, O, R, W> std::ops::Deref for MapWriteRef<V, O, R, W>
where
  O: DerefMut,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  type Target = V;
  #[inline]
  fn deref(&self) -> &Self::Target { (self.map_read)(self.origin_ref.deref()) }
}

impl<V, O, R, W> std::ops::DerefMut for MapWriteRef<V, O, R, W>
where
  O: DerefMut,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { (self.map_write)(self.origin_ref.deref_mut()) }
}

impl<V, O, R, W> RefWrite for MapWriteRef<V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  #[inline]
  fn forget_modifies(&mut self) -> bool { self.origin_ref.forget_modifies() }
}

impl<R, S, F> RenderTarget for MapReader<R, S, F>
where
  R: Render,
  S: StateReader,
  F: FnOnce(&S::Value) -> &R + Copy + 'static,
{
  type Target = R;

  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V {
    let v = self.read();
    f(&*v)
  }
}

impl<V: Render, R: StateReader, F: FnOnce(&R::Value) -> &V + Copy + 'static> RenderBuilder
  for MapReader<V, R, F>
{
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { RenderProxy::new(self).widget_build(ctx) }
}

impl<V, W, RM, WM> RenderBuilder for MapWriter<V, W, RM, WM>
where
  V: Render,
  W: StateWriter,
  RM: FnOnce(&W::Value) -> &V + Copy + 'static,
  WM: FnOnce(&mut W::Value) -> &mut V + Copy,
{
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    // we needn't keep a writer as render widget, keep a reader is enough.
    MapReader {
      origin_reader: self.origin_writer.clone_reader(),
      map_fn: self.map_reader,
    }
    .widget_build(ctx)
  }
}
