use ribir_algo::Sc;

use super::*;
use crate::widget::*;

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<O, W> {
  origin: O,
  splitter: W,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  ref_count: Sc<Cell<usize>>,
}

impl<O, W> Drop for SplittedWriter<O, W> {
  fn drop(&mut self) {
    if self.ref_count.get() == 1 {
      let mut notifier = self.notifier.clone();
      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      let _ = AppCtx::spawn_local(async move {
        notifier.unsubscribe();
      });
    }
  }
}

impl<V, O, W> StateReader for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  type Value = V;
  type OriginReader = O;
  type Reader = MapWriterAsReader<O::Reader, W>;

  #[track_caller]
  fn read(&self) -> ReadRef<Self::Value> {
    ReadRef::mut_as_ref_map(self.origin.read(), &self.splitter)
  }

  #[inline]
  fn clone_reader(&self) -> Self::Reader {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.splitter.clone() }
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

impl<V, O, W> StateWatcher for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  fn raw_modifies(&self) -> CloneableBoxOp<'static, ModifyScope, std::convert::Infallible> {
    self.notifier.raw_modifies().box_it()
  }
}

impl<V, O, W> StateWriter for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  type Writer = SplittedWriter<O::Writer, W>;
  type OriginWriter = O;

  #[inline]
  fn write(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.write()) }

  #[inline]
  fn silent(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.silent()) }

  #[inline]
  fn shallow(&self) -> WriteRef<Self::Value> { self.split_ref(self.origin.shallow()) }

  fn clone_writer(&self) -> Self::Writer {
    SplittedWriter {
      origin: self.origin.clone_writer(),
      splitter: self.splitter.clone(),
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      ref_count: self.ref_count.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, O, W> WriterControl for SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  #[inline]
  fn batched_modifies(&self) -> &Cell<ModifyScope> { &self.batched_modify }

  #[inline]
  fn notifier(&self) -> &Notifier { &self.notifier }

  #[inline]
  fn dyn_clone(&self) -> Box<dyn WriterControl> { Box::new(self.clone_writer()) }
}

impl<V, O, W> SplittedWriter<O, W>
where
  Self: 'static,
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone,
{
  pub(super) fn new(origin: O, mut_map: W) -> Self {
    Self {
      origin,
      splitter: mut_map,
      notifier: Notifier::default(),
      batched_modify: <_>::default(),
      ref_count: Sc::new(Cell::new(1)),
    }
  }

  #[track_caller]
  fn split_ref<'a>(&'a self, mut orig: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    let modify_scope = orig.modify_scope;

    // the origin mark as a silent write, because split writer not effect the origin
    // state in ribir framework level. But keep notify in the data level.
    assert!(!orig.modified);
    orig.modify_scope.remove(ModifyScope::FRAMEWORK);
    orig.modified = true;
    let value =
      ValueMutRef { inner: (self.splitter)(&mut orig.value), borrow: orig.value.borrow.clone() };

    WriteRef { value, modified: false, modify_scope, control: self }
  }
}

impl<V, O, W> RenderBuilder for SplittedWriter<O, W>
where
  O: StateWriter,
  W: Fn(&mut O::Value) -> PartData<V> + Clone + 'static,
  V: Render,
{
  fn build(self, ctx: &BuildCtx) -> Widget {
    MapWriterAsReader { origin: self.origin.clone_reader(), part_map: self.splitter.clone() }
      .build(ctx)
  }
}

impl<S, F> IntoWidgetStrict<RENDER> for SplittedWriter<S, F>
where
  Self: StateWriter,
  <Self as StateReader>::Reader: IntoWidget<RENDER>,
{
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget { self.clone_reader().into_widget(ctx) }
}

impl<S, F> IntoWidgetStrict<COMPOSE> for SplittedWriter<S, F>
where
  Self: StateWriter,
  <Self as StateReader>::Value: Compose,
{
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget { Compose::compose(self).build(ctx) }
}

impl<S, F, C> IntoWidgetStrict<COMPOSE_CHILD> for SplittedWriter<S, F>
where
  Self: StateWriter,
  <Self as StateReader>::Value: ComposeChild<Child = Option<C>>,
{
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    ComposeChild::compose_child(self, None).build(ctx)
  }
}
