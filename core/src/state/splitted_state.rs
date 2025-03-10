use super::*;
use crate::widget::*;

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<O, W> {
  origin: O,
  splitter: W,
  info: Sc<WriterInfo>,
}

impl<O, W> Drop for SplittedWriter<O, W> {
  fn drop(&mut self) { self.info.dec_writer() }
}

impl<V: ?Sized, O, W> StateReader for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartMut<V> + Clone,
{
  type Value = V;
  type Reader = MapReader<O::Reader, WriterMapReaderFn<W>>;

  #[track_caller]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.splitter)
  }

  #[inline]
  fn clone_boxed_reader(&self) -> Box<dyn StateReader<Value = Self::Value>> {
    Box::new(self.clone_reader())
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapReader {
      origin: self.origin.clone_reader(),
      part_map: WriterMapReaderFn(self.splitter.clone()),
    }
  }
}

impl<V: ?Sized, O, W> StateWatcher for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartMut<V> + Clone,
{
  type Watcher = Watcher<Self::Reader>;

  fn into_reader(self) -> Result<Self::Reader, Self> {
    if self.info.writer_count.get() == 1 { Ok(self.clone_reader()) } else { Err(self) }
  }

  #[inline]
  fn clone_boxed_watcher(&self) -> Box<dyn StateWatcher<Value = Self::Value>> {
    Box::new(self.clone_watcher())
  }
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, std::convert::Infallible> {
    self.info.notifier.raw_modifies().box_it()
  }

  #[inline]
  fn clone_watcher(&self) -> Watcher<Self::Reader> {
    Watcher::new(self.clone_reader(), self.raw_modifies())
  }
}

impl<V: ?Sized, O, W> StateWriter for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartMut<V> + Clone,
{
  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.write()) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.silent()) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.shallow()) }

  #[inline]
  fn clone_boxed_writer(&self) -> Box<dyn StateWriter<Value = Self::Value>> {
    Box::new(self.clone_writer())
  }

  fn clone_writer(&self) -> Self {
    self.info.inc_writer();
    SplittedWriter {
      origin: self.origin.clone_writer(),
      splitter: self.splitter.clone(),
      info: self.info.clone(),
    }
  }
}

impl<V: ?Sized, O, W> SplittedWriter<O, W>
where
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartMut<V> + Clone,
{
  pub(super) fn new(origin: O, mut_map: W) -> Self {
    Self { origin, splitter: mut_map, info: Sc::new(WriterInfo::new()) }
  }

  #[track_caller]
  fn split_ref<'a>(&'a self, mut orig: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    let modify_scope = orig.modify_scope;

    // the origin mark as a silent write, because split writer not effect the origin
    // state in ribir framework level. But keep notify in the data level.
    assert!(!orig.modified);
    orig.modify_scope.remove(ModifyScope::FRAMEWORK);
    orig.modified = true;
    let value = ValueMutRef {
      inner: (self.splitter)(&mut orig.value).inner,
      borrow: orig.value.borrow.clone(),
    };

    WriteRef { value, modified: false, modify_scope, info: &self.info }
  }
}

impl<'w, S, F> IntoWidgetStrict<'w, RENDER> for SplittedWriter<S, F>
where
  Self: StateWriter<Value: Render + Sized> + 'w,
{
  fn into_widget_strict(self) -> Widget<'w> { WriterRender(self).into_widget() }
}
