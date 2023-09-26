use super::{MapReadRef, MapReader, ModifyScope, Notifier, RefWrite, StateReader, StateWriter};
use crate::context::AppCtx;
use ribir_algo::Sc;
use rxrust::{
  ops::box_it::BoxOp,
  prelude::{ObservableItem, Observer},
  subject::Subject,
  subscription::Subscription,
};
use std::{any::Any, cell::Cell, rc::Rc};

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<V, O, R, W>
where
  O: StateWriter,
  R: FnOnce(&O::Value) -> &V + Copy,
  W: FnOnce(&mut O::Value) -> &mut V + Copy,
{
  origin_writer: O,
  reader: R,
  writer: W,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  connect_guard: Rc<Box<dyn Any>>,
}

impl<V, O, R, W> StateReader for SplittedWriter<V, O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: FnOnce(&O::Value) -> &V + Copy,
  W: FnOnce(&mut O::Value) -> &mut V + Copy,
{
  type Value = V;
  type OriginReader = O;
  type Reader = MapReader<V, O::Reader, R>;

  type Ref<'a> = MapReadRef<V, O::Ref<'a>, R> where Self: 'a;

  #[inline]
  fn read(&'_ self) -> Self::Ref<'_> { MapReadRef::new(self.origin_writer.read(), self.reader) }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader::new(self.origin_writer.clone_reader(), self.reader)
  }

  #[inline]
  fn origin_reader(&self) -> &Self::OriginReader { &self.origin_writer }

  #[inline]
  fn modifies(&self) -> BoxOp<'static, ModifyScope, std::convert::Infallible> {
    self.notifier.modifies()
  }

  #[inline]
  fn raw_modifies(&self) -> Subject<'static, ModifyScope, std::convert::Infallible> {
    self.notifier.raw_modifies()
  }
}

impl<V, O, R, W> StateWriter for SplittedWriter<V, O, R, W>
where
  Self: 'static,
  O: StateWriter,
  R: FnOnce(&O::Value) -> &V + Copy,
  W: FnOnce(&mut O::Value) -> &mut V + Copy,
{
  type Writer = SplittedWriter<V, O::Writer, R, W>;
  type OriginWriter = O;
  type RefWrite<'a> = SplittedWriteRef<'a, V, O::RefWrite<'a>, R, W>;
  #[inline]
  fn write(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::BOTH) }
  #[inline]
  fn silent(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::DATA) }
  #[inline]
  fn shallow(&'_ self) -> Self::RefWrite<'_> { self.write_ref(ModifyScope::FRAMEWORK) }

  #[inline]
  fn clone_writer(&self) -> Self::Writer {
    SplittedWriter {
      origin_writer: self.origin_writer.clone_writer(),
      reader: self.reader,
      writer: self.writer,
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      connect_guard: self.connect_guard.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin_writer }
}

pub struct SplittedWriteRef<'a, V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  origin_ref: O,
  modify_scope: ModifyScope,
  batched_modify: &'a Sc<Cell<ModifyScope>>,
  notifier: &'a Notifier,
  reader_fn: R,
  writer_fn: W,
}

impl<'a, V, O, R, W> std::ops::Deref for SplittedWriteRef<'a, V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  type Target = V;

  #[inline]
  fn deref(&self) -> &Self::Target { (self.reader_fn)(&*self.origin_ref) }
}

impl<'a, V, O, R, W> std::ops::DerefMut for SplittedWriteRef<'a, V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { (self.writer_fn)(&mut *self.origin_ref) }
}

impl<'a, V, O, R, W> RefWrite for SplittedWriteRef<'a, V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  #[inline]
  fn forget_modifies(&mut self) -> bool { self.origin_ref.forget_modifies() }
}

impl<V, O, R, W> SplittedWriter<V, O, R, W>
where
  O: StateWriter,
  R: FnOnce(&O::Value) -> &V + Copy,
  W: FnOnce(&mut O::Value) -> &mut V + Copy,
{
  pub(super) fn new(origin_writer: O, reader: R, writer: W) -> Self {
    let notifier = Notifier::default();
    let c_modifier = notifier.clone();

    let h = origin_writer
      .raw_modifies()
      .subscribe(move |v| c_modifier.raw_modifies().next(v))
      .unsubscribe_when_dropped();

    Self {
      origin_writer,
      reader,
      writer,
      notifier,
      batched_modify: <_>::default(),
      connect_guard: Rc::new(Box::new(h)),
    }
  }

  fn write_ref(&'_ self, scope: ModifyScope) -> SplittedWriteRef<'_, V, O::RefWrite<'_>, R, W> {
    SplittedWriteRef {
      origin_ref: self.origin_writer.write(),
      modify_scope: scope,
      batched_modify: &self.batched_modify,
      notifier: &self.notifier,
      reader_fn: self.reader,
      writer_fn: self.writer,
    }
  }
}

impl<'a, V, O, R, W> Drop for SplittedWriteRef<'a, V, O, R, W>
where
  O: RefWrite,
  R: FnOnce(&O::Target) -> &V + Copy,
  W: FnOnce(&mut O::Target) -> &mut V + Copy,
{
  fn drop(&mut self) {
    if !self.origin_ref.forget_modifies() {
      return;
    }

    let scope = self.modify_scope;
    let batched_modify = self.batched_modify.get();
    if batched_modify.is_empty() && !scope.is_empty() {
      self.batched_modify.set(scope);

      let mut subject = self.notifier.raw_modifies();
      let batched_modify = self.batched_modify.clone();
      AppCtx::spawn_local(async move {
        let scope = batched_modify.replace(ModifyScope::empty());
        subject.next(scope);
      })
      .unwrap();
    } else {
      self.batched_modify.set(batched_modify | scope);
    }
  }
}
