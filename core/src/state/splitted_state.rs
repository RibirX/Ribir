use super::{
  MapReader, ModifyScope, Notifier, ReadRef, StateReader, StateWriter, WriteRef, WriterControl,
};
use crate::prelude::AppCtx;
use crate::{
  context::BuildCtx,
  widget::{Render, RenderBuilder, Widget},
};
use ribir_algo::Sc;
use rxrust::{observable::ObservableExt, ops::box_it::BoxOp, prelude::BoxIt};
use std::{
  cell::{Cell, RefMut},
  time::Instant,
};

/// A writer splitted writer from another writer, and has its own notifier.
pub struct SplittedWriter<O, R, W, C> {
  origin: O,
  map: R,
  mut_map: W,
  checker: Sc<C>,
  notifier: Notifier,
  batched_modify: Sc<Cell<ModifyScope>>,
  last_modified: Sc<Cell<Instant>>,
  ref_count: Sc<Cell<usize>>,
}

/// The SplitChecker will be bind to the SplittedWriter or SplittedReader.
/// The method of is_valid will be call before the visit to the Value of
/// StateReader or StateWriter, by method of read() and write().
/// the default SplitChecker to the SplittedWriter is VolatileSplit, see
/// [VolatileSplit], and you can also set to AlwaysValidSplit(see
/// [AlwaysValidSplit]), or set to your custom checker by set_checker of
/// StateWriter
pub trait SplitChecker<P: ?Sized> {
  fn is_valid<S: StateReader<Value = P>>(&self, src: &S) -> bool;
}

/// The Default SplitChecker. It will return false if the origin writer has been
/// changed or if the origin writer is invalid, and the continue used will cause
/// panic.
/// We set the SplittedWriter default with VolatileSplit checker to prevent the
/// unexpected bug if the SplittedWriter is mapped in not stable way, e.x: map
/// by index to  vector
/// ### Example of SplittedWriter to Vec item.
/// the follow example will panic to exposed the unexpected usage to visit to
/// user Tom.
/// ``` should_panic
/// use ribir_core::prelude::*;
/// #[derive(Debug)]
/// struct User {
///   name: &'static str,
///   //..
/// }
/// let src = Stateful::new(vec![User { name: "Lily" }, User { name: "Lucy" }, User { name: "Tom" }]);
/// let lucy_idx = 1;
/// let lucy = src.split_writer(move |v| &v[lucy_idx], move |v| &mut v[lucy_idx]);
/// src.write().remove(0);
/// println!("lucy's info {:?}", *lucy.read());
/// ```
pub struct VolatileSplit {
  create_at: Instant,
}

impl Default for VolatileSplit {
  fn default() -> Self { VolatileSplit { create_at: Instant::now() } }
}

impl<P: ?Sized> SplitChecker<P> for VolatileSplit {
  fn is_valid<S: StateReader<Value = P>>(&self, p: &S) -> bool {
    self.create_at > p.time_stamp() && p.is_valid()
  }
}

/// The AlwaysValidSplit SplitChecker. It will always return true if the
/// OriginWriter is valid.
pub struct AlwaysValidSplit {}
impl<P: ?Sized> SplitChecker<P> for AlwaysValidSplit {
  fn is_valid<S: StateReader<Value = P>>(&self, p: &S) -> bool { p.is_valid() }
}

impl<O, R, W, C> Drop for SplittedWriter<O, R, W, C> {
  fn drop(&mut self) {
    if self.ref_count.get() == 1 {
      let mut notifier = self.notifier.clone();
      // we use an async task to unsubscribe to wait the batched modifies to be
      // notified.
      AppCtx::spawn_local(async move {
        notifier.unsubscribe();
      })
      .unwrap();
    }
  }
}

pub struct SplittedReader<S, F, C> {
  origin: S,
  map: F,
  checker: Sc<C>,
  notifier: Notifier,
  last_modified: Sc<Cell<Instant>>,
}

macro_rules! splitted_reader_impl {
  () => {
    type Value = V;
    type OriginReader = O;
    type Reader = SplittedReader<O::Reader, R, C>;

    #[track_caller]
    fn read(&self) -> ReadRef<Self::Value> {
      assert!(self.is_valid(), "A splitted reader is invalid.");
      ReadRef::map(self.origin.read(), &self.map)
    }

    #[inline]
    fn clone_reader(&self) -> Self::Reader {
      SplittedReader {
        origin: self.origin.clone_reader(),
        map: self.map.clone(),
        notifier: self.notifier.clone(),
        checker: self.checker.clone(),
        last_modified: self.last_modified.clone(),
      }
    }

    #[inline]
    fn is_valid(&self) -> bool { self.checker.is_valid(self.origin_reader()) }

    #[inline]
    fn origin_reader(&self) -> &Self::OriginReader { &self.origin }

    #[inline]
    fn time_stamp(&self) -> Instant { self.last_modified.get() }

    #[inline]
    fn raw_modifies(&self) -> BoxOp<'static, ModifyScope, std::convert::Infallible> {
      let this = self.clone_reader();
      self
        .notifier
        .raw_modifies()
        .filter(move |_| this.is_valid())
        .box_it()
    }

    #[inline]
    fn try_into_value(self) -> Result<Self::Value, Self>
    where
      Self::Value: Sized,
    {
      Err(self)
    }
  };
}

impl<V, O, R, C> StateReader for SplittedReader<O, R, C>
where
  Self: 'static,
  V: ?Sized,
  O: StateReader,
  R: Fn(&O::Value) -> &V + Clone,
  C: SplitChecker<O::Value>,
{
  splitted_reader_impl!();
}

impl<V, O, R, W, C> SplittedWriter<O, R, W, C>
where
  Self: 'static,
  V: ?Sized,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  /// set your custom checker to replace the default checker,
  /// See[`VolatileSplit`].
  /// ### Example of set_checker to SplittedWriter
  /// ``` rust
  /// use ribir_core::prelude::*;
  /// #[derive(Default)]
  /// struct App {
  ///    name: String,
  ///    data: Vec<String>,
  ///    //...
  /// }
  /// let app = Stateful::new(App::default());
  /// let data = app.split_writer(|a| &a.data, |a| &mut a.data).set_checker(AlwaysValidSplit {});
  /// app.write().name = "ribir".to_string();
  ///
  /// // the use of data will cause panic if not set the checker to AlwaysValidSplit. See [`VolatileSplit`]
  /// println!("data len {}", data.read().len());
  /// ```
  pub fn set_checker<C2>(self, checker: C2) -> SplittedWriter<O::Writer, R, W, C2>
  where
    C2: SplitChecker<O::Value>,
  {
    assert!(
      self.ref_count.get() == 1,
      "SplittedWriter's set_checker should be call after create and before used"
    );
    SplittedWriter {
      origin: self.origin.clone_writer(),
      map: self.map.clone(),
      mut_map: self.mut_map.clone(),
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      checker: Sc::new(checker),
      last_modified: self.last_modified.clone(),
      ref_count: self.ref_count.clone(),
    }
  }
}

impl<C, V, O, R, W> StateReader for SplittedWriter<O, R, W, C>
where
  Self: 'static,
  V: ?Sized,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
  C: SplitChecker<O::Value>,
{
  splitted_reader_impl!();
}

impl<V, O, R, W, C> StateWriter for SplittedWriter<O, R, W, C>
where
  Self: 'static,
  V: ?Sized,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
  C: SplitChecker<O::Value>,
{
  type Writer = SplittedWriter<O::Writer, R, W, C>;
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
      map: self.map.clone(),
      mut_map: self.mut_map.clone(),
      notifier: self.notifier.clone(),
      batched_modify: self.batched_modify.clone(),
      checker: self.checker.clone(),
      last_modified: self.last_modified.clone(),
      ref_count: self.ref_count.clone(),
    }
  }

  #[inline]
  fn origin_writer(&self) -> &Self::OriginWriter { &self.origin }
}

impl<V, O, R, W, C> WriterControl for SplittedWriter<O, R, W, C>
where
  Self: 'static,
  V: ?Sized,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  C: SplitChecker<O::Value>,
  W: Fn(&mut O::Value) -> &mut V + Clone,
{
  #[inline]
  fn last_modified_stamp(&self) -> &Cell<Instant> { &self.last_modified }

  #[inline]
  fn batched_modifies(&self) -> &Cell<ModifyScope> { &self.batched_modify }

  #[inline]
  fn notifier(&self) -> &Notifier { &self.notifier }

  #[inline]
  fn dyn_clone(&self) -> Box<dyn WriterControl> { Box::new(self.clone_writer()) }
}

impl<V, O, R, W, C> SplittedWriter<O, R, W, C>
where
  Self: 'static,
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone,
  W: Fn(&mut O::Value) -> &mut V + Clone,
  C: SplitChecker<O::Value>,
  V: ?Sized,
{
  pub(super) fn new(origin: O, map: R, mut_map: W, checker: C) -> Self {
    let create_at = Instant::now();
    Self {
      origin,
      map,
      mut_map,
      notifier: Notifier::default(),
      batched_modify: <_>::default(),
      checker: Sc::new(checker),
      last_modified: Sc::new(Cell::new(create_at)),
      ref_count: Sc::new(Cell::new(1)),
    }
  }

  #[track_caller]
  fn split_ref<'a>(&'a self, mut orig: WriteRef<'a, O::Value>) -> WriteRef<'a, V> {
    assert!(self.is_valid(), "A splitted writer is invalid.");
    let modify_scope = orig.modify_scope;

    // the origin mark as a silent write, because split writer not effect the origin
    // state in ribir framework level. But keep notify in the data level.
    assert!(!orig.modified);
    orig.modify_scope.remove(ModifyScope::FRAMEWORK);
    orig.modified = true;

    let value = orig
      .value
      .take()
      .map(|orig| RefMut::map(orig, &self.mut_map));

    WriteRef {
      value,
      modified: false,
      modify_scope,
      control: self,
    }
  }
}

impl<V, O, R, W, C> RenderBuilder for SplittedWriter<O, R, W, C>
where
  O: StateWriter,
  R: Fn(&O::Value) -> &V + Clone + 'static,
  W: Fn(&mut O::Value) -> &mut V + Clone,
  C: SplitChecker<O::Value>,
  V: Render,
{
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    MapReader {
      origin: self.origin.clone_reader(),
      map: self.map.clone(),
    }
    .widget_build(ctx)
  }
}

#[cfg(test)]
mod tests {
  use crate::test_helper::TestWindow;
  use crate::{prelude::*, reset_test_env};
  use crate::{state::Stateful, test_helper::split_value};

  #[test]
  fn ref_invalid_split_writer() {
    reset_test_env!();
    let src = Stateful::new(vec![1]);
    let splitted = src.split_writer(|v| &v[0], |v| &mut v[0]);
    let (res_reader, res_writer) = split_value(0);
    let (trigger_reader, trigger_writer) = split_value(0);
    let w = fn_widget! {
      watch!(*$splitted + *$trigger_reader)
        .subscribe(move |v| *res_writer.write() = v);
      @Void {}
    };

    let mut wnd = TestWindow::new(w);
    *trigger_writer.write() += 1;
    wnd.draw_frame();
    assert_eq!(*res_reader.read(), 2);

    src.write().pop();
    wnd.draw_frame();
    assert_eq!(*res_reader.read(), 2);

    *trigger_writer.write() += 1;
    wnd.draw_frame();
    assert_eq!(*res_reader.read(), 2);
  }
}
