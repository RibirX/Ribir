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

impl<S, M, V: ?Sized> StateReader for MapReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: MapReaderFn<S::Value, Output = V>,
{
  type Value = V;
  type Reader = MapReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.part_map.call(self.origin.read()) }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
  }
}

impl<V: ?Sized, S, M> StateReader for MapWriter<S, M>
where
  Self: 'static,
  S: StateWriter,
  M: Fn(&mut S::Value) -> PartMut<V> + Clone,
{
  type Value = V;
  type Reader = MapReader<S::Reader, WriterMapReaderFn<M>>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.part_map)
  }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin: self.origin.clone_reader(),
      part_map: WriterMapReaderFn(self.part_map.clone()),
    }
  }
}

impl<V: ?Sized, W, M> StateWatcher for MapWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartMut<V> + Clone,
{
  type Watcher = Watcher<Self::Reader>;

  fn into_reader(self) -> Result<Self::Reader, Self> {
    let Self { origin, part_map } = self;
    match origin.into_reader() {
      Ok(origin) => Ok(MapReader { origin, part_map: WriterMapReaderFn(part_map) }),
      Err(origin) => Err(Self { origin, part_map }),
    }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.origin.raw_modifies()
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<V: ?Sized, W, M> StateWriter for MapWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartMut<V> + Clone,
{
  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.write(), &self.part_map) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { WriteRef::map(self.origin.silent(), &self.part_map) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> {
    WriteRef::map(self.origin.shallow(), &self.part_map)
  }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  #[inline]
  fn clone_writer(&self) -> Self {
    MapWriter { origin: self.origin.clone_writer(), part_map: self.part_map.clone() }
  }
}

impl<V: ?Sized, S, F> RenderProxy for MapReader<S, F>
where
  Self: 'static,
  S: StateReader,
  F: Fn(&S::Value) -> PartRef<V> + Clone,
  V: Render,
{
  #[inline]
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.read() }
}

impl<'w, S, F> IntoWidget<'w, RENDER> for MapWriter<S, F>
where
  Self: StateWriter<Value: Render + Sized>,
{
  fn into_widget(self) -> Widget<'w> { WriterRender(self).into_widget() }
}

trait MapReaderFn<Input: ?Sized>: Clone {
  type Output: ?Sized;
  fn call<'a>(&self, input: ReadRef<'a, Input>) -> ReadRef<'a, Self::Output>;
}

impl<Input: ?Sized, Output: ?Sized, F> MapReaderFn<Input> for F
where
  F: Fn(&Input) -> PartRef<Output> + Clone,
{
  type Output = Output;
  fn call<'a>(&self, input: ReadRef<'a, Input>) -> ReadRef<'a, Self::Output> {
    ReadRef::map(input, self)
  }
}

impl<Input: ?Sized, Output: ?Sized, F> MapReaderFn<Input> for WriterMapReaderFn<F>
where
  F: Fn(&mut Input) -> PartMut<Output> + Clone,
{
  type Output = Output;
  fn call<'a>(&self, input: ReadRef<'a, Input>) -> ReadRef<'a, Self::Output> {
    ReadRef::mut_as_ref_map(input, &self.0)
  }
}

#[derive(Clone)]
pub struct WriterMapReaderFn<F>(pub(crate) F);
