use super::*;

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader.
pub struct PartReader<S, F> {
  pub(super) origin: S,
  pub(super) part_map: F,
}

pub struct PartWriter<W, WM> {
  pub(super) origin: W,
  pub(super) part_map: WM,
  pub(super) path: PartialPath,
  pub(super) include_partial: bool,
}

impl<S, M, V: ?Sized> StateReader for PartReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: MapReaderFn<S::Value, Output = V>,
{
  type Value = V;
  type Reader = PartReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.part_map.call(self.origin.read()) }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    PartReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
  }
}

impl<V: ?Sized, S, M> StateReader for PartWriter<S, M>
where
  Self: 'static,
  S: StateWriter,
  M: Fn(&mut S::Value) -> PartMut<V> + Clone,
{
  type Value = V;
  type Reader = PartReader<S::Reader, WriterMapReaderFn<M>>;

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
    PartReader {
      origin: self.origin.clone_reader(),
      part_map: WriterMapReaderFn(self.part_map.clone()),
    }
  }
}

impl<V: ?Sized, W, M> StateWatcher for PartWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartMut<V> + Clone,
{
  type Watcher = Watcher<Self::Reader>;

  fn into_reader(self) -> Result<Self::Reader, Self> {
    let Self { origin, part_map, path, include_partial } = self;
    match origin.into_reader() {
      Ok(origin) => Ok(PartReader { origin, part_map: WriterMapReaderFn(part_map) }),
      Err(origin) => Err(Self { origin, part_map, path, include_partial }),
    }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    let modifies = self.origin.raw_modifies();
    let path = self.path.clone();
    let include_partial = self.include_partial;

    if !self.path.is_empty() {
      modifies
        .filter(move |info| info.path_matches(&path, include_partial))
        .box_it()
    } else {
      modifies
    }
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<V: ?Sized, W, M> StateWriter for PartWriter<W, M>
where
  Self: 'static,
  W: StateWriter,
  M: Fn(&mut W::Value) -> PartMut<V> + Clone,
{
  fn write(&self) -> WriteRef<Self::Value> {
    let mut w = WriteRef::map(self.origin.write(), &self.part_map);
    w.path = &self.path;
    w
  }

  fn silent(&self) -> WriteRef<Self::Value> {
    let mut w = WriteRef::map(self.origin.silent(), &self.part_map);
    w.path = &self.path;
    w
  }

  fn shallow(&self) -> WriteRef<Self::Value> {
    let mut w = WriteRef::map(self.origin.shallow(), &self.part_map);
    w.path = &self.path;
    w
  }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  #[inline]
  fn clone_writer(&self) -> Self {
    PartWriter {
      origin: self.origin.clone_writer(),
      part_map: self.part_map.clone(),
      path: self.path.clone(),
      include_partial: self.include_partial,
    }
  }

  fn include_partial_writers(mut self, include: bool) -> Self {
    self.include_partial = include;
    self
  }

  fn scope_path(&self) -> &PartialPath { &self.path }
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
