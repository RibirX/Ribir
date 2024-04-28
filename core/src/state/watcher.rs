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

impl<R: StateReader> StateReader for Watcher<R> {
  type Value = R::Value;
  type Reader = R::Reader;
  type OriginReader = R::OriginReader;

  #[inline]
  fn read(&self) -> ReadRef<Self::Value> { self.reader.read() }

  #[inline]
  fn clone_reader(&self) -> Self::Reader { self.reader.clone_reader() }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { self.reader.origin_reader() }

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
  #[inline]
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, Infallible> {
    self.modifies_observable.clone()
  }
}
