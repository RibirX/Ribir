use super::*;
use crate::{render_helper::RenderProxy, widget::*};

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

impl<S, V: ?Sized, M> StateReader for MapReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: Fn(&S::Value) -> PartData<V> + Clone,
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

impl<S, V: ?Sized, M> StateReader for MapWriterAsReader<S, M>
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

impl<V: ?Sized, S, M> StateReader for MapWriter<S, M>
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

impl<V: ?Sized, W, M> StateWatcher for MapWriter<W, M>
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

impl<V: ?Sized, W, M> StateWriter for MapWriter<W, M>
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

impl<V: ?Sized, S, F> RenderProxy for MapReader<S, F>
where
  Self: 'static,
  S: StateReader,
  F: Fn(&S::Value) -> PartData<V> + Clone,
  V: Render,
{
  type R = V;

  type Target<'r> = ReadRef<'r, V>
  where
    Self: 'r;

  #[inline]
  fn proxy(&self) -> Self::Target<'_> { self.read() }
}

impl<V: ?Sized, S, F> RenderProxy for MapWriterAsReader<S, F>
where
  Self: 'static,
  S: StateReader,
  F: Fn(&mut S::Value) -> PartData<V> + Clone,
  V: Render,
{
  type R = V;

  type Target<'r> = ReadRef<'r, V>

  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.read() }
}

impl<'w, S, F> IntoWidgetStrict<'w, RENDER> for MapWriter<S, F>
where
  Self: 'static,
  Self: StateReader + 'w,
  <Self as StateReader>::Reader: IntoWidget<'w, RENDER>,
{
  fn into_widget_strict(self) -> Widget<'w> { self.clone_reader().into_widget() }
}

impl<S, F> IntoWidgetStrict<'static, COMPOSE> for MapWriter<S, F>
where
  Self: StateWriter + 'static,
  <Self as StateReader>::Value: Compose,
{
  fn into_widget_strict(self) -> Widget<'static> { Compose::compose(self) }
}
