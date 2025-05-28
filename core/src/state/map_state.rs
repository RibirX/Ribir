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
  pub(super) id: Option<PartialId>,
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
    let Self { origin, part_map, id } = self;
    match origin.into_reader() {
      Ok(origin) => Ok(PartReader { origin, part_map: WriterMapReaderFn(part_map) }),
      Err(origin) => Err(Self { origin, part_map, id }),
    }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyInfo, Infallible> {
    if let Some(id) = &self.id {
      let id = id.clone();
      self
        .origin
        .raw_modifies()
        .filter_map(move |s| {
          let ModifyInfo { partial, scope } = s;
          if partial
            .as_ref()
            .is_some_and(|p| p.first() == Some(&id))
          {
            let mut partial = partial.unwrap();
            partial.remove(0);
            Some(ModifyInfo { partial: Some(partial), scope })
          } else {
            None
          }
        })
        .box_it()
    } else {
      self.origin.raw_modifies()
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
  #[inline]
  fn write(&self) -> WriteRef<Self::Value> {
    WriteRef::map(self.origin.write(), &self.part_map, self.id.as_ref())
  }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> {
    WriteRef::map(self.origin.silent(), &self.part_map, self.id.as_ref())
  }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> {
    WriteRef::map(self.origin.shallow(), &self.part_map, self.id.as_ref())
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
      id: self.id.clone(),
    }
  }
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
