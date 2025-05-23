use super::*;

/// A state reader that map a reader to another by applying a function on the
/// value. This reader is the same reader with the origin reader.
pub struct MapReader<S, F> {
  pub(super) origin: S,
  pub(super) part_map: F,
}

impl<S, M, V: ?Sized> StateReader for MapReader<S, M>
where
  Self: 'static,
  S: StateReader,
  M: Fn(&S::Value) -> PartRef<V> + Clone,
{
  type Value = V;
  type Reader = MapReader<S::Reader, M>;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { ReadRef::map(self.origin.read(), &self.part_map) }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader { origin: self.origin.clone_reader(), part_map: self.part_map.clone() }
  }
}

#[derive(Clone)]
pub struct MapState<W, M> {
  pub(crate) orig: W,
  pub(crate) map: M,
}

impl<O, W, U: ?Sized> StateValue for MapState<O, W>
where
  O: StateValue + 'static + Clone,
  W: Fn(&mut O::Value) -> PartMut<U> + Clone + 'static,
{
  type Value = U;

  fn read(&self) -> ReadRef<U> { ReadRef::mut_as_ref_map(self.orig.read(), &self.map) }

  fn try_into_value(self) -> std::result::Result<U, Self>
  where
    U: Sized,
  {
    Err(self)
  }

  fn write(&self) -> ValueMutRef<U> { ValueMutRef::map(self.orig.write(), &self.map) }

  fn clone_box(&self) -> Box<dyn StateValue<Value = Self::Value>> { Box::new(self.clone()) }
}
