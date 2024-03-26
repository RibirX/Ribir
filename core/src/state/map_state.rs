use crate::{
  context::BuildCtx,
  render_helper::{RenderProxy, RenderTarget},
  ticker::Instant,
  widget::{Render, RenderBuilder, Widget},
};

use super::{ModifyScope, ReadRef, StateReader, StateWriter, WriteRef};
use rxrust::ops::box_it::BoxOp;
use std::{cell::RefMut, convert::Infallible};

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader, It's also have
/// the same modifier with the origin state.
pub struct MapReader<S, F> {
  pub(super) origin: S,
  pub(super) map: F,
}

pub struct MapWriter<W, RM, WM> {
  pub(super) origin: W,
  pub(super) map: RM,
  pub(super) mut_map: WM,
}

macro_rules! impl_reader_trivial_methods {
  () => {
    type Value = V;
    type OriginReader = S;
    type Reader = MapReader<S::Reader, R>;

    #[inline]
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
    fn time_stamp(&self) -> Instant { self.origin.time_stamp() }

    #[inline]
    fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.origin.raw_modifies() }

    #[inline]
    fn try_into_value(self) -> Result<Self::Value, Self>
    where
      Self::Value: Sized,
    {
      Err(self)
    }
  };
}

impl<S, V, R> StateReader for MapReader<S, R>
where
  Self: 'static,
  V: ?Sized,
  S: StateReader,
  R: Fn(&S::Value) -> &V + Clone + 'static,
{
  impl_reader_trivial_methods!();
}

impl<V, S, R, W> StateReader for MapWriter<S, R, W>
where
  Self: 'static,
  V: ?Sized,
  S: StateWriter,
  R: Fn(&S::Value) -> &V + Clone,
  W: Fn(&mut S::Value) -> &mut V + Clone,
{
  impl_reader_trivial_methods!();
}

impl<V, W, RM, WM> StateWriter for MapWriter<W, RM, WM>
where
  Self: 'static,
  V: ?Sized,
  W: StateWriter,
  RM: Fn(&W::Value) -> &V + Clone,
  WM: Fn(&mut W::Value) -> &mut V + Clone,
{
  type Writer = MapWriter<W::Writer, RM, WM>;
  type OriginWriter = W;

  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { self.map_ref(self.origin.write()) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { self.map_ref(self.origin.silent()) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { self.map_ref(self.origin.shallow()) }

  #[inline]
  fn clone_writer(&self) -> Self::Writer {
    MapWriter {
      origin: self.origin.clone_writer(),
      map: self.map.clone(),
      mut_map: self.mut_map.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, S, F> RenderTarget for MapReader<S, F>
where
  S: StateReader,
  F: Fn(&S::Value) -> &V + Clone + 'static,
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
  F: Fn(&S::Value) -> &V + Clone + 'static,
  V: Render,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { RenderProxy::new(self).build(ctx) }
}

impl<V, S, RM, WM> RenderBuilder for MapWriter<S, RM, WM>
where
  Self: 'static,
  S: StateWriter,
  RM: Fn(&S::Value) -> &V + Clone,
  WM: Fn(&mut S::Value) -> &mut V + Clone,
  V: Render,
{
  #[inline]
  fn build(self, ctx: &BuildCtx) -> Widget { self.clone_reader().build(ctx) }
}

impl<V, S, RM, WM> MapWriter<S, RM, WM>
where
  Self: 'static,
  V: ?Sized,
  S: StateWriter,
  RM: Fn(&S::Value) -> &V + Clone,
  WM: Fn(&mut S::Value) -> &mut V + Clone,
{
  fn map_ref<'a>(&'a self, mut orig: WriteRef<'a, S::Value>) -> WriteRef<'a, V>
  where
    WM: Fn(&mut S::Value) -> &mut V,
  {
    let value = orig
      .value
      .take()
      .map(|orig| RefMut::map(orig, &self.mut_map));

    WriteRef {
      value,
      modified: false,
      modify_scope: orig.modify_scope,
      control: orig.control,
    }
  }
}
