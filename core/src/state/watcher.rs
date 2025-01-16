use std::convert::Infallible;

use rxrust::ops::box_it::CloneableBoxOp;

use crate::prelude::*;

pub struct Watcher<R> {
  reader: R,
  modifies_observable: CloneableBoxOp<'static, ModifyScope, Infallible>,
}

impl<R> Watcher<R> {
  pub fn new(
    reader: R, modifies_observable: CloneableBoxOp<'static, ModifyScope, Infallible>,
  ) -> Self {
    Self { reader, modifies_observable }
  }
}

impl<R> From<Watcher<Reader<R>>> for Reader<R> {
  fn from(w: Watcher<Reader<R>>) -> Self { w.reader }
}

impl<R: StateReader> StateReader for Watcher<R> {
  type Value = R::Value;
  type Reader = R::Reader;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.reader.read() }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.reader.clone_reader() }

  #[inline]
  fn try_into_value(self) -> Result<Self::Value, Self>
  where
    Self::Value: Sized,
  {
    let Self { reader, modifies_observable } = self;
    reader
      .try_into_value()
      .map_err(|reader| Self { reader, modifies_observable })
  }
}

impl<R: StateReader> StateWatcher for Watcher<R> {
  type Watcher = Watcher<R::Reader>;

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }

  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.modifies_observable.clone()
  }

  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}
