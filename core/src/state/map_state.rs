use crate::{
  context::BuildCtx,
  render_helper::{RenderProxy, RenderTarget},
  widget::{Render, RenderBuilder, Widget},
};

use super::{ModifyScope, ReadRef, StateReader, StateWriter, WriteRef};
use rxrust::{ops::box_it::BoxOp, subject::Subject};
use std::convert::Infallible;

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader, It's also have
/// the same modifier with the origin state.
// Keep the `V` as the first generic, so the user know the actual value type
// when ide hints.
pub struct MapReader<V, S, F>
where
  S: StateReader,
  F: Fn(&S::Value) -> &V + Clone + 'static,
{
  pub(super) origin: S,
  pub(super) map: F,
}

pub struct MapWriter<V, W, RM, WM>
where
  W: StateWriter,
  RM: Fn(&W::Value) -> &V + Clone,
  WM: Fn(&mut W::Value) -> &mut V + Clone,
{
  pub(super) origin: W,
  pub(super) map: RM,
  pub(super) mut_map: WM,
}

macro_rules! impl_reader_trivial_methods {
  () => {
    #[inline]
    fn origin_reader(&self) -> &Self::OriginReader { &self.origin }

    #[inline]
    fn modifies(&self) -> BoxOp<'static, ModifyScope, Infallible> { self.origin.modifies() }

    #[inline]
    fn raw_modifies(&self) -> Subject<'static, ModifyScope, Infallible> {
      self.origin.raw_modifies()
    }

    #[inline]
    fn try_into_value(self) -> Result<Self::Value, Self> { Err(self) }
  };
}

impl<R, V, F> StateReader for MapReader<V, R, F>
where
  Self: 'static,
  R: StateReader,
  F: Fn(&R::Value) -> &V + Clone + 'static,
{
  type Value = V;
  type OriginReader = R;
  type Reader = MapReader<V, R::Reader, F>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { ReadRef::map(self.origin.read(), &self.map) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin: self.origin.clone_reader(),
      map: self.map.clone(),
    }
  }

  impl_reader_trivial_methods!();
}

impl<V, W, RM, WM> StateReader for MapWriter<V, W, RM, WM>
where
  Self: 'static,
  W: StateWriter,
  RM: Fn(&W::Value) -> &V + Clone,
  WM: Fn(&mut W::Value) -> &mut V + Clone,
{
  type Value = V;
  type OriginReader = W;
  type Reader = MapReader<V, W::Reader, RM>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { ReadRef::map(self.origin.read(), &self.map) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin: self.origin.clone_reader(),
      map: self.map.clone(),
    }
  }

  impl_reader_trivial_methods!();
}

impl<V, W, RM, WM> StateWriter for MapWriter<V, W, RM, WM>
where
  Self: 'static,
  W: StateWriter,
  RM: Fn(&W::Value) -> &V + Clone,
  WM: Fn(&mut W::Value) -> &mut V + Clone,
{
  type Writer = MapWriter<V, W::Writer, RM, WM>;
  type OriginWriter = W;

  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.write(), &self.mut_map) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.silent(), &self.mut_map) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.shallow(), &self.mut_map) }

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

impl<V, S, F> RenderTarget for MapReader<V, S, F>
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

impl<V, S, F> RenderBuilder for MapReader<V, S, F>
where
  S: StateReader,
  F: Fn(&S::Value) -> &V + Clone + 'static,
  V: Render,
{
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { RenderProxy::new(self).widget_build(ctx) }
}

impl<V, S, RM, WM> RenderBuilder for MapWriter<V, S, RM, WM>
where
  Self: 'static,
  S: StateWriter,
  RM: Fn(&S::Value) -> &V + Clone,
  WM: Fn(&mut S::Value) -> &mut V + Clone,
  V: Render,
{
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self.clone_reader().widget_build(ctx) }
}
