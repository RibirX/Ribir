use std::{
  cell::{Cell, UnsafeCell},
  convert::Infallible,
  ops::RangeInclusive,
  ptr::NonNull,
};

use ribir_algo::Sc;
use rxrust::ops::box_it::BoxOp;
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::{prelude::*, render_helper::PureRender};

pub type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

/// Trait used to create a widget from a pipe value.
pub(crate) trait PipeWidget<const M: usize>: IntoWidget<'static, M> {
  type Widget;
}

/// Trait used to create a widget from a option pipe value.
pub(crate) trait OptionPipeWidget<const M: usize> {
  type Widget;
  fn option_to_widget(self) -> Widget<'static>;
}

impl<F: FnOnce() -> W + 'static, W: IntoWidget<'static, M>, const M: usize> PipeWidget<M>
  for FnWidget<'static, F, W, M>
{
  type Widget = W;
}

impl PipeWidget<FN> for BoxFnWidget<'static> {
  type Widget = Widget<'static>;
}

impl PipeWidget<FN> for GenWidget {
  type Widget = Widget<'static>;
}

impl<const M: usize, W> OptionPipeWidget<M> for W
where
  W: PipeWidget<M>,
{
  type Widget = W::Widget;
  fn option_to_widget(self) -> Widget<'static> { self.into_widget() }
}

impl<const M: usize, W> OptionPipeWidget<M> for Option<W>
where
  W: PipeWidget<M>,
{
  type Widget = W::Widget;
  fn option_to_widget(self) -> Widget<'static> {
    self.map_or_else(|| Void.into_widget(), |f| f.into_widget())
  }
}

/// A trait for a value that can be subscribed its continuous modifies.
pub trait Pipe: 'static {
  type Value;

  /// Maps an `Pipe<Value=T>` to `Pipe<Value=T>` by applying a function to the
  /// continuous value
  fn map<R, F: FnMut(Self::Value) -> R + 'static>(self, f: F) -> MapPipe<R, Self, F>
  where
    Self: Sized,
  {
    MapPipe::new(self, f)
  }

  /// Further operations can be chained on the pipe value stream by applying the
  /// `f` function to the final value stream upon subscription. This operation
  /// is lazy and will not execute the `f` function until the pipe is subscribed
  /// to.
  ///
  /// # Unsafe
  /// If your pipe is used as a widget, avoid chaining any asynchronous
  /// operations, as this may disrupt the building process.
  fn value_chain<F>(self, f: F) -> FinalChain<Self::Value, Self, F>
  where
    Self: Sized,
    F: FnOnce(ValueStream<Self::Value>) -> ValueStream<Self::Value> + 'static,
  {
    FinalChain { source: self, f, _marker: PhantomData }
  }

  /// Unzip the `Pipe` into its inner value and the stream of changes for that
  /// value.
  ///
  /// - *scope*: specifies the scope of the modifications to be emitted by the
  ///   stream.
  /// - *init*: If this is a pipe widget, an `PipeWidgetBuildInit` needs to be
  ///   provided to init the build context.
  fn unzip(
    self, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>);

  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>);
}

/// A trait object type for `Pipe`, help to store a concrete `Pipe`
/// or just a value.
///
/// This type not implement `Pipe` trait to avoid boxing the `Pipe` twice and
/// has a better conversion from `Pipe` to `BoxPipe`.
///
/// Call `into_pipe` to convert it to a `Pipe` type.
#[derive(ChildOfCompose)]
pub struct BoxPipe<V>(Box<dyn Pipe<Value = V>>);

pub struct MapPipe<V, S, F> {
  source: S,
  f: F,
  _marker: PhantomData<V>,
}

pub struct FinalChain<V, S, F> {
  source: S,
  f: F,
  _marker: PhantomData<V>,
}

impl<V: 'static> BoxPipe<V> {
  #[inline]
  pub fn value(v: V) -> Self { Self(Box::new(ValuePipe(v))) }

  #[inline]
  pub fn pipe(p: Box<dyn Pipe<Value = V>>) -> Self { Self(p) }

  #[inline]
  pub fn into_pipe(self) -> Box<dyn Pipe<Value = V>> { self.0 }
}

pub(crate) trait InnerPipe: Pipe + Sized {
  fn build_single<const M: usize>(self) -> Widget<'static>
  where
    Self::Value: OptionPipeWidget<M> + 'static,
  {
    let f = move || {
      let pipe_node = PipeNode::empty_node();

      let tree_ptr = BuildCtx::get().tree_ptr();
      let init = PipeWidgetBuildInit::new_with_tree(pipe_node.clone(), tree_ptr);
      let (w, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(init));
      w.option_to_widget().on_build(move |w| {
        pipe_node.init_for_single(w);

        let c_pipe_node = pipe_node.clone();
        let u = modifies.subscribe(move |(_, w)| {
          let old = pipe_node.dyn_info().host_id();
          let old_node = pipe_node.take_data();
          let without_ctx = BuildCtx::try_get().is_none();
          if without_ctx {
            BuildCtx::set_for(old, unsafe { NonNull::new_unchecked(tree_ptr) });
          }
          let ctx = BuildCtx::get_mut();
          let new = ctx.build(w.option_to_widget());
          let tree = ctx.tree_mut();
          pipe_node.transplant_to_new(old_node, new, tree);

          query_outside_infos(new, &pipe_node, tree)
            .for_each(|node| node.dyn_info_mut().single_replace(old, new));

          old.insert_after(new, tree);
          old.dispose_subtree(tree);
          new.on_mounted_subtree(tree);

          tree.dirty_marker().mark(new, DirtyPhase::Layout);

          if without_ctx {
            BuildCtx::clear();
          }
        });
        c_pipe_node.attach_subscription(u);
      })
    };
    f.into_widget()
  }

  fn build_multi<'w, const M: usize, I>(self) -> Vec<Widget<'w>>
  where
    Self::Value: FnOnce() -> I,
    I: IntoIterator,
    <I as IntoIterator>::Item: IntoWidget<'w, M>,
  {
    let node = PipeNode::empty_node();
    let mut init = PipeWidgetBuildInit::new(node.clone());
    let (m, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(init.clone()));
    let mut iter = m().into_iter().map(|w| w.into_widget());

    let pipe_node = node.clone();
    let first = iter
      .next()
      .unwrap_or_else(|| Void {}.into_widget());
    let first = first.on_build(move |id| {
      pipe_node.init(id, GenRange::Multi(vec![id]));
      let tree_ptr = BuildCtx::get().tree_ptr();
      init.set_tree(tree_ptr);

      let c_pipe_node = pipe_node.clone();

      let u = modifies.subscribe(move |(_, m)| {
        let old = match &pipe_node.dyn_info().gen_range {
          GenRange::Multi(m) => m.clone(),
          _ => unreachable!(),
        };

        let old_node = pipe_node.take_data();
        let without_ctx = BuildCtx::try_get().is_none();
        if without_ctx {
          BuildCtx::set_for(pipe_node.dyn_info().host_id(), unsafe {
            NonNull::new_unchecked(tree_ptr)
          });
        }

        let ctx = BuildCtx::get_mut();
        let mut new = vec![];
        for w in m().into_iter().map(|w| w.into_widget()) {
          let id = ctx.build(w);

          new.push(id);
        }
        if new.is_empty() {
          new.push(ctx.build(Void.into_widget()));
        }

        let dummy = ctx.build(Void {}.into_widget());
        let tree = ctx.tree_mut();

        if new[0] != old[0] {
          pipe_node.transplant_to_new(old_node, new[0], tree);
        } else {
          pipe_node.replace_data(old_node);
        }

        // Replacing the old widget ID with new widgets for the external pipe node of
        // the current pipe. The self dynamic information will be updated at
        // the end,including the widget id and the key. So the key is only
        // used to the same level.
        query_outside_infos(new[0], &pipe_node, tree).for_each(|node| {
          node
            .dyn_info_mut()
            .multi_replace(&old, new.iter().cloned())
        });

        old[0].insert_after(dummy, tree);
        new
          .iter()
          .rev()
          .for_each(|w| dummy.insert_after(*w, tree));

        old.iter().for_each(|id| id.dispose_subtree(tree));

        new.iter().for_each(|id| {
          id.on_mounted_subtree(tree);
          tree.dirty_marker().mark(*id, DirtyPhase::Layout);
        });

        dummy.dispose_subtree(tree);

        pipe_node.dyn_info_mut().gen_range = GenRange::Multi(new);

        if without_ctx {
          BuildCtx::clear();
        }
      });

      c_pipe_node.attach_subscription(u);
    });

    let mut widgets = vec![first];
    for w in iter {
      let pipe_node = node.clone();
      let w = w.on_build(move |id| {
        match &mut pipe_node.dyn_info_mut().gen_range {
          GenRange::Multi(m) => m.push(id),
          _ => unreachable!(),
        };

        let tree = BuildCtx::get_mut().tree_mut();
        if has_pipe_node(id, tree) {
          // We need to associate the parent information with the children pipe so that
          // when the child pipe is regenerated, it can update the parent pipe information
          // accordingly.
          id.attach_data(Box::new(QueryFilter::only_self(pipe_node)), tree);
        }
      });

      widgets.push(w);
    }

    widgets
  }

  fn into_parent_widget<const M: usize>(self) -> Widget<'static>
  where
    Self: Sized,
    Self::Value: OptionPipeWidget<M> + 'static,
  {
    let f = move || {
      let node = PipeNode::empty_node();
      let init = PipeWidgetBuildInit::new_with_tree(node.clone(), BuildCtx::get().tree_ptr());
      let (w, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(init));

      w.option_to_widget().on_build(move |p| {
        let tree: &mut WidgetTree = BuildCtx::get_mut().tree_mut();
        let leaf = p.single_leaf(tree);
        node.init(p, GenRange::ParentOnly(p..=leaf));

        // We need to associate the parent information with the pipe of leaf widget,
        // when the leaf pipe is regenerated, it can update the parent pipe information
        // accordingly.
        if leaf.contain_type::<PipeNode>(tree) {
          leaf.attach_data(Box::new(node.clone()), tree);
        };

        let pipe_node = node.clone();
        let u = modifies.subscribe(move |(_, w)| {
          let (top, bottom) = match &pipe_node.dyn_info().gen_range {
            GenRange::ParentOnly(p) => p.clone().into_inner(),
            _ => unreachable!(),
          };

          let old_node = pipe_node.take_data();

          let without_ctx = BuildCtx::try_get().is_none();
          if without_ctx {
            BuildCtx::set_for(pipe_node.dyn_info().host_id(), unsafe {
              NonNull::new_unchecked(tree)
            });
          }

          let p = BuildCtx::get_mut().build(w.option_to_widget());
          let tree = BuildCtx::get_mut().tree_mut();
          pipe_node.transplant_to_new(old_node, p, tree);

          let new_rg = p..=p.single_leaf(tree);
          let children: SmallVec<[WidgetId; 1]> = bottom.children(tree).collect();
          for c in children {
            new_rg.end().append(c, tree);
          }

          query_outside_infos(p, &pipe_node, tree).for_each(|node| {
            node
              .dyn_info_mut()
              .single_range_replace(&(top..=bottom), &new_rg);
          });

          top.insert_after(p, tree);
          top.dispose_subtree(tree);

          let mut w = p;
          loop {
            w.on_widget_mounted(tree);
            if w == *new_rg.end() {
              break;
            }
            if let Some(c) = w.first_child(tree) {
              w = c;
            } else {
              break;
            }
          }

          tree.dirty_marker().mark(p, DirtyPhase::Layout);

          if without_ctx {
            BuildCtx::clear();
          }
        });

        node.attach_subscription(u);
      })
    };
    f.into_widget()
  }
}

impl<S: Pipe, V, F: FnMut(S::Value) -> V> MapPipe<V, S, F> {
  #[inline]
  pub fn new(source: S, f: F) -> Self { Self { source, f, _marker: PhantomData } }
}

pub struct ModifiesPipe(BoxOp<'static, ModifyScope, Infallible>);

impl ModifiesPipe {
  #[inline]
  pub fn new(modifies: BoxOp<'static, ModifyScope, Infallible>) -> Self { Self(modifies) }
}

impl Pipe for ModifiesPipe {
  type Value = ModifyScope;

  #[inline]
  fn unzip(
    self, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let stream = self
      .0
      .filter(move |s| s.contains(scope))
      .map(|s| (s, s));

    let stream = if let Some(init) = init {
      let source = stream
        .sample(AppCtx::frame_ticks().clone())
        .priority(init.clone());

      PipeWidgetContextOp { source, init }.box_it()
    } else {
      stream.box_it()
    };

    (ModifyScope::empty(), stream)
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, updater: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, updater)
  }
}

impl<V: 'static> Pipe for Box<dyn Pipe<Value = V>> {
  type Value = V;

  #[inline]
  fn unzip(
    self, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.box_unzip(scope, init)
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).box_unzip(scope, init)
  }
}

impl<V, S, F> Pipe for MapPipe<V, S, F>
where
  Self: 'static,
  S: Pipe,
  F: FnMut(S::Value) -> V,
{
  type Value = V;

  fn unzip(
    self, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f, .. } = self;
    let (v, stream) = source.unzip(scope, init);
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, init)
  }
}

impl<V, S, F> Pipe for FinalChain<V, S, F>
where
  Self: 'static,
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V>,
{
  type Value = V;

  fn unzip(
    self, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, f, .. } = self;
    let (v, stream) = source.unzip(scope, init);
    (v, f(stream))
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, init: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, init)
  }
}

/// A pipe that never changes, help to construct a pipe from a value.
struct ValuePipe<V>(V);

impl<V: 'static> Pipe for ValuePipe<V> {
  type Value = V;

  #[inline]
  fn unzip(
    self, _: ModifyScope, _: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (self.0, observable::empty().box_it())
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<PipeWidgetBuildInit>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.unzip(scope, priority)
  }
}

impl<T: Pipe> InnerPipe for T {}

impl<V, S, F, const M: usize> IntoWidget<'static, M> for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: OptionPipeWidget<M>,
{
  fn into_widget(self) -> Widget<'static> { self.build_single() }
}

impl<V, S, F, const M: usize> IntoWidget<'static, M> for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: OptionPipeWidget<M>,
{
  fn into_widget(self) -> Widget<'static> { self.build_single() }
}

impl<const M: usize, V> IntoWidget<'static, M> for Box<dyn Pipe<Value = V>>
where
  V: OptionPipeWidget<M> + 'static,
{
  fn into_widget(self) -> Widget<'static> { self.build_single() }
}

/// `PipeNode` just use to wrap a `Box<dyn Render>`, and provide a choice to
/// change the inner `Box<dyn Render>` by `UnsafeCell` at a safe time --
/// although it is stored using a reference counting pointer, its logic
/// guarantees that it is uniquely accessed in the entire call stack and use it
/// locally that not worry about the borrow conflict.
///
/// It's transparent except the `Pipe` widget.
///
/// We use a `PipeNode` wrap the widget node of the pipe, so we can only
/// replace the dynamic part come from the pipe, and keep the static data
/// attached to this node. For example, we attached the unsubscribe handle of
/// the pipe to the first node, and user can attach `key` or `listener` to the
/// widget after `Pipe::build_widget` call.
#[derive(Clone)]
pub(crate) struct PipeNode(Sc<UnsafeCell<InnerPipeNode>>);

struct InnerPipeNode {
  data: Box<dyn RenderQueryable>,
  dyn_info: DynWidgetsInfo,
}

#[derive(Debug)]
pub(crate) struct DynWidgetsInfo {
  pub(crate) gen_range: GenRange,
}

#[derive(Debug)]
pub enum GenRange {
  Single(WidgetId),
  Multi(Vec<WidgetId>),
  ParentOnly(RangeInclusive<WidgetId>),
}

impl DynWidgetsInfo {
  pub(crate) fn new(range: GenRange) -> DynWidgetsInfo { DynWidgetsInfo { gen_range: range } }

  pub(crate) fn single_replace(&mut self, old: WidgetId, new: WidgetId) {
    match &mut self.gen_range {
      GenRange::Single(id) => {
        assert_eq!(*id, old, "For single pipe node, the logic pipe child must be same `PipeNode`.");
        *id = new;
      }
      GenRange::Multi(m) => {
        if let Some(idx) = m.iter().position(|w| w == &old) {
          m[idx] = new;
        }
      }
      GenRange::ParentOnly(p) => {
        if p.start() == &old {
          *p = new..=*p.end();
        }
        if p.end() == &old {
          *p = *p.start()..=new;
        }
      }
    }
  }

  pub(crate) fn single_range_replace(
    &mut self, old: &RangeInclusive<WidgetId>, new: &RangeInclusive<WidgetId>,
  ) {
    match &mut self.gen_range {
      GenRange::Single(id) => *id = *new.start(),
      GenRange::Multi(m) => {
        let p = *old.start();
        if let Some(idx) = m.iter().position(|w| w == &p) {
          m[idx] = *new.start();
        }
      }
      GenRange::ParentOnly(p) => {
        if p.start() == old.start() {
          *p = *new.start()..=*p.end();
        }
        if p.end() == old.end() {
          *p = *p.start()..=*new.end();
        }
      }
    }
  }

  fn multi_replace(&mut self, old: &[WidgetId], new: impl Iterator<Item = WidgetId>) {
    match &mut self.gen_range {
      GenRange::Single(_) => unreachable!("Single pipe node never have multi pipe child."),
      GenRange::Multi(m) => {
        if let Some(from) = m.iter().position(|w| &old[0] == w) {
          let to = m
            .iter()
            .position(|w| &old[old.len() - 1] == w)
            .expect("must include");
          m.splice(from..=to, new);
        }
      }
      GenRange::ParentOnly(_) => {
        unreachable!("Single parent node never have multi pipe child.")
      }
    }
  }

  pub(crate) fn host_id(&self) -> WidgetId {
    match &self.gen_range {
      GenRange::Single(id) => *id,
      GenRange::Multi(m) => *m.first().unwrap(),
      GenRange::ParentOnly(p) => *p.start(),
    }
  }
}

impl PipeNode {
  pub(crate) fn empty_node() -> Self {
    let gen_range = GenRange::Single(BuildCtx::get().tree().dummy_id());
    let dyn_info = DynWidgetsInfo::new(gen_range);
    let inner = InnerPipeNode { data: Box::new(PureRender(Void)), dyn_info };
    Self(Sc::new(UnsafeCell::new(inner)))
  }

  pub(crate) fn init_for_single(&self, w: WidgetId) { self.init(w, GenRange::Single(w)); }

  pub(crate) fn init(&self, id: WidgetId, range: GenRange) {
    self.dyn_info_mut().gen_range = range;
    let node = self.clone();
    id.wrap_node(BuildCtx::get_mut().tree_mut(), |r| {
      node.as_mut().data = r;
      Box::new(node)
    });
  }

  pub(crate) fn dyn_info(&self) -> &DynWidgetsInfo { &self.as_ref().dyn_info }

  pub(crate) fn dyn_info_mut(&self) -> &mut DynWidgetsInfo { &mut self.as_mut().dyn_info }

  // Remove the old widget so that the new widget build logic cannot access it
  // anymore.
  pub(crate) fn take_data(&self) -> Box<dyn RenderQueryable> {
    self.replace_data(Box::new(PureRender(Void)))
  }

  pub(crate) fn replace_data(&self, new: Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable> {
    std::mem::replace(&mut self.as_mut().data, new)
  }

  fn transplant_to_new(
    &self, old_node: Box<dyn RenderQueryable>, new_id: WidgetId, tree: &mut WidgetTree,
  ) {
    let old = self.dyn_info().host_id();

    let [old, new] = tree.get_many_mut(&[old, new_id]);
    std::mem::swap(old, new);

    new.update_track_id(new_id);

    std::mem::swap(&mut self.as_mut().data, old);
    *old = old_node;
  }

  fn as_ref(&self) -> &InnerPipeNode {
    // safety: see the `PipeNode` document.
    unsafe { &*self.0.get() }
  }

  #[allow(clippy::mut_from_ref)]
  fn as_mut(&self) -> &mut InnerPipeNode {
    // safety: see the `PipeNode` document.
    unsafe { &mut *self.0.get() }
  }

  /// Attach a subscription to host widget of the `PipeNode`, and the
  /// subscription will be unsubscribed when the `PipeNode` dropped.
  fn attach_subscription(self, u: impl Subscription + 'static) {
    let tree = BuildCtx::get_mut().tree_mut();
    let node = self.as_mut();
    let id = node.dyn_info.host_id();
    // if the subscription is closed, we can cancel and unwrap the `PipeNode`
    // immediately.
    if u.is_closed() {
      let v = std::mem::replace(&mut node.data, Box::new(PureRender(Void)));
      *id.get_node_mut(tree).unwrap() = v;
    } else {
      id.attach_anonymous_data(u.unsubscribe_when_dropped(), tree)
    }
  }
}

fn has_pipe_node(w: WidgetId, tree: &WidgetTree) -> bool { w.query_ref::<PipeNode>(tree).is_some() }

fn query_outside_infos<'l>(
  id: WidgetId, to: &'l PipeNode, tree: &'l WidgetTree,
) -> impl Iterator<Item = QueryRef<'l, PipeNode>> {
  let mut hit = false;
  id.query_all_iter::<PipeNode>(tree)
    .rev()
    .take_while(move |info| {
      if hit {
        false
      } else {
        hit = Sc::ptr_eq(&info.0, &to.0);
        true
      }
    })
}

fn pipe_priority_value(node: &PipeNode, tree: &WidgetTree) -> i64 {
  let id = node.dyn_info().host_id();
  let depth = id.ancestors(tree).count() as i64;
  let embed = query_outside_infos(id, node, tree).count() as i64;

  depth << 32 | embed
}

impl Query for PipeNode {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    let p = self.as_ref();
    p.data.query_all(query_id, out);
    if query_id == &QueryId::of::<Self>() {
      out.push(QueryHandle::new(self))
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    let p = self.as_ref();
    p.data.query_all_write(query_id, out);
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    let p = self.as_ref();
    if query_id == &QueryId::of::<Self>() {
      Some(QueryHandle::new(self))
    } else {
      p.data.query(query_id)
    }
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.as_ref().data.query_write(query_id)
  }

  fn queryable(&self) -> bool { true }
}

impl Render for PipeNode {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.as_ref().data.perform_layout(clamp, ctx)
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> { self.as_ref().data.visual_box(ctx) }

  fn paint(&self, ctx: &mut PaintingCtx) { self.as_ref().data.paint(ctx) }

  fn only_sized_by_parent(&self) -> bool {
    // A pipe node is always sized by its parent because it can generate any widget.
    false
  }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self.as_ref().data.hit_test(ctx, pos)
  }

  fn get_transform(&self) -> Option<Transform> { self.as_ref().data.get_transform() }

  fn dirty_phase(&self) -> DirtyPhase { self.as_ref().data.dirty_phase() }
}

#[derive(Clone)]
pub struct PipeWidgetBuildInit {
  node: PipeNode,
  tree: Sc<Cell<*mut WidgetTree>>,
}

impl PipeWidgetBuildInit {
  fn new_with_tree(node: PipeNode, tree: *mut WidgetTree) -> Self {
    let mut p = Self::new(node);
    p.set_tree(tree);
    p
  }

  fn new(node: PipeNode) -> Self { Self { node, tree: Sc::new(Cell::new(std::ptr::null_mut())) } }

  fn set_tree(&mut self, tree: *mut WidgetTree) {
    assert!(self.tree.get().is_null());
    self.tree.set(tree);
  }

  fn tree(&self) -> Option<&WidgetTree> {
    let tree = self.tree.get();
    (!tree.is_null()).then(|| unsafe { &*tree })
  }
}

impl Priority for PipeWidgetBuildInit {
  fn priority(&mut self) -> i64 {
    self
      .tree()
      .map_or(-1, |tree| pipe_priority_value(&self.node, tree))
  }

  fn queue(&mut self) -> Option<&PriorityTaskQueue> {
    self.tree().map(|tree| {
      let wnd = tree.window();
      let queue = wnd.priority_task_queue();
      // Safety: This trait is only used within this crate, and we can ensure that
      // the window is valid when utilizing the `PriorityTaskQueue`.
      unsafe { std::mem::transmute(queue) }
    })
  }
}

struct PipeWidgetContextOp<S> {
  source: S,
  init: PipeWidgetBuildInit,
}

struct PipeWidgetContextObserver<O> {
  observer: O,
  init: PipeWidgetBuildInit,
}

impl<Item: 'static, Err: 'static, O, S> Observable<Item, Err, O> for PipeWidgetContextOp<S>
where
  O: Observer<Item, Err> + 'static,
  S: Observable<Item, Err, PipeWidgetContextObserver<O>> + 'static,
{
  type Unsub = S::Unsub;

  fn actual_subscribe(self, observer: O) -> Self::Unsub {
    let Self { source, init } = self;
    source.actual_subscribe(PipeWidgetContextObserver { observer, init })
  }
}

impl<Item, Err, S> ObservableExt<Item, Err> for PipeWidgetContextOp<S> where
  S: ObservableExt<Item, Err>
{
}

impl<Item, Err, O> Observer<Item, Err> for PipeWidgetContextObserver<O>
where
  O: Observer<Item, Err>,
{
  fn next(&mut self, value: Item) {
    let Self { observer, init } = self;
    let wid = init.node.dyn_info().host_id();
    let tree = NonNull::new(init.tree.get()).expect("Tree must not be null.");

    // Initialize the build context
    let old = init.node.take_data();
    BuildCtx::set_for(wid, tree);
    init.node.replace_data(old);

    observer.next(value);

    // Clear the build context immediately after the downstream process.
    BuildCtx::clear();
  }

  fn error(self, err: Err) { self.observer.error(err); }

  fn complete(self) { self.observer.complete(); }

  fn is_finished(&self) -> bool { self.observer.is_finished() }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_as_root() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_writer();
    let w = fn_widget! {
      let p = pipe! { fn_widget!{ @MockBox { size: *$size }}};
      @$p { @Void {} }
    };
    let wnd = TestWindow::new(w);
    let tree = wnd.tree_mut();
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let new_ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 2);

    assert_eq!(ids[1], new_ids[1]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn expr_widget_with_declare_child() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_writer();
    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        @ {
          let p = pipe! { fn_widget! {MockBox { size: *$size }}};
          @$p { @Void {} }
        }
      }
    };
    let wnd = TestWindow::new(w);
    let tree = wnd.tree_mut();
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let new_ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 3);

    assert_eq!(ids[0], new_ids[0]);
    assert_eq!(ids[2], new_ids[2]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_mounted_new() {
    reset_test_env!();

    let v = Stateful::new(vec![1, 2, 3]);
    let new_cnt = Stateful::new(0);
    let drop_cnt = Stateful::new(0);

    let c_v = v.clone_writer();
    let c_new_cnt = new_cnt.clone_reader();
    let c_drop_cnt = drop_cnt.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!($v.clone()).map(move |v| {
            move || {
                v.into_iter().map(move |_| fn_widget!{
                @MockBox{
                  size: Size::zero(),
                  on_mounted: move |_| *$new_cnt.write() += 1,
                  on_disposed: move |_| *$drop_cnt.write() += 1
                }
              })
            }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 3);
    assert_eq!(*c_drop_cnt.read(), 0);

    c_v.write().push(4);
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 7);
    assert_eq!(*c_drop_cnt.read(), 3);

    c_v.write().pop();
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 10);
    assert_eq!(*c_drop_cnt.read(), 7);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_in_pipe() {
    reset_test_env!();
    let (p, w_p) = split_value(false);
    let (c, w_c) = split_value(false);
    let (mnt_cnt, w_mnt_cnt) = split_value(0);

    let w = fn_widget! {
      pipe!(*$p).map(move |_| fn_widget!{
        @MockBox {
          size: Size::zero(),
          on_mounted: move |_| *$w_mnt_cnt.write() +=1,
          @{
            pipe!(*$c).map(move |_| fn_widget!{
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| *$w_mnt_cnt.write() +=1,
              }
            })
          }
        }
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*mnt_cnt.read(), 2);

    // trigger the parent update
    *w_p.write() = true;
    // then trigger the child update.
    *w_c.write() = true;

    wnd.draw_frame();
    assert_eq!(*mnt_cnt.read(), 4);

    // old pipe should be unsubscribed.
    *w_p.write() = true;
    *w_c.write() = true;
    wnd.draw_frame();
    assert_eq!(*mnt_cnt.read(), 6);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn delay_drop_widgets() {
    reset_test_env!();

    #[derive(Default, Clone)]
    struct Task {
      mounted: u32,
      pin: bool,
      paint_cnt: Rc<Cell<u32>>,
      layout_cnt: Rc<Cell<u32>>,
      trigger: u32,
      wid: Option<WidgetId>,
    }

    fn build(task: Stateful<Task>) -> Widget<'static> {
      fn_widget! {
       @TaskWidget {
          keep_alive: pipe!($task.pin),
          layout_cnt: pipe!($task.layout_cnt.clone()),
          paint_cnt: pipe!($task.paint_cnt.clone()),
          trigger: pipe!($task.trigger),
          on_mounted: move |ctx| {
            $task.write().mounted += 1;
            $task.write().wid = Some(ctx.id);
          },
          on_disposed: move |ctx| {
            let wid = $task.write().wid.take();
            assert_eq!(wid, Some(ctx.id));
          }
        }
      }
      .into_widget()
    }

    #[derive(Declare)]
    struct TaskWidget {
      trigger: u32,
      paint_cnt: Rc<Cell<u32>>,
      layout_cnt: Rc<Cell<u32>>,
    }

    impl Render for TaskWidget {
      fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
        self.layout_cnt.set(self.layout_cnt.get() + 1);
        Size::new(1., 1.)
      }

      fn paint(&self, _: &mut PaintingCtx) { self.paint_cnt.set(self.paint_cnt.get() + 1); }
    }

    fn child_count(wnd: &Window) -> usize {
      let tree = wnd.tree();
      let root = tree.content_root();
      root.children(tree).count()
    }

    let tasks = (0..3)
      .map(|_| Stateful::new(Task::default()))
      .collect::<Vec<_>>();
    let tasks = Stateful::new(tasks);
    let c_tasks = tasks.clone_watcher();
    let w = fn_widget! {
      @MockMulti {
        @ { pipe!{
          move || $c_tasks.iter().map(|t| build(t.clone_writer())).collect::<Vec<_>>()
        }}
      }
    };

    let mut wnd = TestWindow::new(w);
    let mut removed = vec![];

    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 3);

    // the first pined widget will still paint it
    tasks.read()[0].write().pin = true;
    removed.push(tasks.write().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 2);
    assert_eq!(removed[0].read().paint_cnt.get(), 2);

    // the remove pined widget will paint and no layout when no changed
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    tasks.read().first().unwrap().write().pin = true;
    removed.push(tasks.write().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 1);
    assert_eq!(removed[0].read().paint_cnt.get(), 3);
    assert_eq!(removed[1].read().paint_cnt.get(), 3);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);

    // the remove pined widget only mark self dirty
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    let second_layout_cnt = removed[1].read().layout_cnt.get();
    let host_layout_cnt = tasks.read()[0].read().layout_cnt.get();
    removed[0].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt + 1);
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().layout_cnt.get(), second_layout_cnt);
    assert_eq!(tasks.read()[0].read().layout_cnt.get(), host_layout_cnt);

    // when unpined, it will no paint anymore
    removed[0].write().pin = false;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 5);

    // after removed, it will no paint and layout anymore
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    removed[0].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 5);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);

    // other pined widget is work fine.
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    let second_layout_cnt = removed[1].read().layout_cnt.get();
    removed[1].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 6);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);
    assert_eq!(removed[1].read().layout_cnt.get(), second_layout_cnt + 1,);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn remove_delay_drop_widgets() {
    reset_test_env!();

    let child = Stateful::new(Some(()));
    let child_destroy_until = Stateful::new(false);
    let grandson = Stateful::new(Some(()));
    let grandson_destroy_until = Stateful::new(false);
    let c_child = child.clone_writer();
    let c_child_destroy_until = child_destroy_until.clone_writer();

    let w = fn_widget! {
      @MockMulti {
        @ { pipe!(*$child).map(move |_| fn_widget!{
          @MockMulti {
            keep_alive: pipe!(!*$child_destroy_until),
            @ { pipe!(*$grandson).map(move |_| fn_widget!{
              @MockBox {
                keep_alive: pipe!(!*$grandson_destroy_until),
                size: Size::zero(),
              }
            })}
          }
        })}
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    let grandson_id = {
      let tree = wnd.tree();
      let root = tree.content_root();
      root
        .first_child(tree)
        .unwrap()
        .first_child(tree)
        .unwrap()
    };

    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(wnd.tree()));

    c_child.write().take();
    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(wnd.tree()));

    *c_child_destroy_until.write() = true;
    wnd.draw_frame();
    assert!(grandson_id.is_dropped(wnd.tree()));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn value_pipe() {
    reset_test_env!();
    let hit = State::value(-1);

    let v = BoxPipe::value(0);
    let (v, s) = v.into_pipe().unzip(ModifyScope::all(), None);

    assert_eq!(v, 0);

    let c_hit = hit.clone_writer();
    let u = s.subscribe(move |_| *c_hit.write() += 1);

    assert_eq!(*hit.read(), -1);
    assert!(u.is_closed());
  }

  #[test]
  fn widget_from_pipe_widget() {
    reset_test_env!();
    let _ = fn_widget! {
      let v = Stateful::new(true);
      let w = pipe!(*$v).map(move |_| fn_widget!{ @ Void {} });
      w.into_widget()
    };
  }

  #[test]
  fn multi_pipe_gen_single_pipe() {
    reset_test_env!();
    let box_count = Stateful::new(1);
    let child_size = Stateful::new(Size::new(1., 1.));
    let c_box_count = box_count.clone_writer();
    let c_child_size = child_size.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$box_count).map(move |v| {
            move || {
              (0..v).map(move |_| fn_widget!{
                pipe!(*$child_size).map(move |size| fn_widget! { @MockBox { size } })
              })
            }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(2., 1.));

    *c_box_count.write() = 2;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(4., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn multi_pipe_gen_pipe_parent_pipe_only() {
    reset_test_env!();
    let box_count = Stateful::new(1);
    let child_size = Stateful::new(Size::new(1., 1.));
    let c_box_count = box_count.clone_writer();
    let c_child_size = child_size.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$box_count).map(move |v| {
            move || {
              (0..v).map(move |_| {
                let pipe_parent = pipe!(*$child_size)
                  .map(move |size| fn_widget!{ @MockBox { size } });
                @$pipe_parent { @Void {} }
              })
            }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(2., 1.));

    *c_box_count.write() = 2;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(4., 1.));
  }

  #[test]
  fn single_pipe_gen_single_pipe() {
    reset_test_env!();
    let pipe_trigger = Stateful::new(0);
    let inner_pipe_trigger = Stateful::new(0);
    let c_pipe_trigger = pipe_trigger.clone_writer();
    let c_inner_pipe_trigger = inner_pipe_trigger.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$pipe_trigger).map(move |w| fn_widget!{
            pipe!(*$inner_pipe_trigger)
              .map(move |h| fn_widget! { @MockBox { size: Size::new(w as f32, h as f32) } })
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn single_pipe_gen_parent_pipe_only() {
    reset_test_env!();
    let (inner, w_inner) = split_value(0);
    let (outer, w_outer) = split_value(0);

    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$outer).map(move |w| fn_widget!{
            let pipe_parent = pipe!(*$inner)
              .map(move |h| fn_widget! {@MockBox { size: Size::new(w as f32, h as f32) } });
            @$pipe_parent { @Void {} }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    // Inner pipe update
    *w_inner.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    // Outer pipe update
    *w_outer.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn parent_pipe_only_gen_single_pipe() {
    reset_test_env!();
    let pipe_trigger = Stateful::new(0);
    let inner_pipe_trigger = Stateful::new(0);
    let c_pipe_trigger = pipe_trigger.clone_writer();
    let c_inner_pipe_trigger = inner_pipe_trigger.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          let p = pipe!(*$pipe_trigger).map(move |w| fn_widget!{
            pipe!(*$inner_pipe_trigger)
              .map(move |h| fn_widget! { @MockBox { size: Size::new(w as f32, h as f32) }})
          });

          @$p { @Void {} }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[test]
  fn fix_pipe_gen_pipe_widget_leak() {
    reset_test_env!();

    let parent = State::value(true);
    let child = State::value(true);
    let hit_count = State::value(0);
    let c_parent = parent.clone_writer();
    let c_child = child.clone_writer();
    let c_hit_count = hit_count.clone_writer();

    let w = fn_widget! {
      pipe!($parent;).map(move |_| fn_widget!{
        pipe!($child;).map(move |_| fn_widget!{
          *$hit_count.write() += 1;
          Void
        })
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*c_hit_count.read(), 1);

    *c_parent.write() = false;
    wnd.draw_frame();
    assert_eq!(*c_hit_count.read(), 2);

    *c_child.write() = false;
    wnd.draw_frame();
    // if the child pipe not reset, the hit count will be 4.
    assert_eq!(*c_hit_count.read(), 3);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn two_pipe_gen_same_render_widget() {
    reset_test_env!();

    let (r1, w1) = split_value(1.);
    let (r2, w2) = split_value(1.);

    let widget = fn_widget! {
      @MockMulti {
        @{
          pipe!(*$r1).map(move |_| fn_widget!{
            pipe!(*$r2)
              .map(move |r| fn_widget!{
                @MockBox {
                  background: Color::YELLOW,
                  size: Size::new(100.0, 10.0 * r + 100.),
                }
            })
          })
        }
      }
    };

    let mut wnd = TestWindow::new(widget);
    wnd.draw_frame();
    *w1.write() += 1.;
    *w2.write() += 1.;
    wnd.draw_frame();
    *w2.write() += 1.;
    *w1.write() += 1.;
    wnd.draw_frame();
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn multi_pipe_gen_pipe_in_order() {
    reset_test_env!();
    let w = Stateful::new(vec![]);
    let w2 = w.clone_writer();
    let widget = fn_widget! {
      @MockMulti {
        @ {
          pipe!($w;).map(move |_| {
            move || {
              $w.silent().push(0);
              (0..10).map(move |idx| {
                pipe!($w;).map(move |_| fn_widget!{
                  $w.silent().push(idx + 1);
                  @MockBox {
                    size: Size::new(1., 1.),
                  }
                })
              })
            }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(widget);
    wnd.draw_frame();
    assert_eq!(&*w2.read(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    w2.write().clear();
    wnd.draw_frame();
    assert_eq!(&*w2.read(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  }

  #[test]
  fn fix_pipe_in_multi_pipe_not_first() {
    reset_test_env!();
    let (m_watcher, m_writer) = split_value(0);
    let (son_watcher, son_writer) = split_value(0);

    let widget = fn_widget! {
      @MockMulti {
        @ {
          pipe!($m_watcher;).map(move |_| {
            move || {[
              Void.into_widget() ,
              pipe!($son_watcher;).map(|_| fn_widget! {@Void{}}).into_widget(),
            ]}
          })
        }
      }
    };

    let mut wnd = TestWindow::new(widget);
    wnd.draw_frame();
    *son_writer.write() += 1;
    wnd.draw_frame();
    // If the parent pipe is not correctly collected, it may cause a panic when
    // refreshing the child widget of the pipe.
    *m_writer.write() += 1;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_in_parent_only_pipe_at_end() {
    reset_test_env!();
    let (m_watcher, m_writer) = split_value(0);
    let (son_watcher, son_writer) = split_value(0);

    let widget = fn_widget! {
      let p = @ {
        pipe!($m_watcher;).map(move |_| fn_widget!{
          // margin is static, but its child MockBox is a pipe.
          let p = pipe!($son_watcher;).map(|_| fn_widget! { MockBox { size: Size::zero() }});
          let mut obj = FatObj::new(p);
          obj.margin(EdgeInsets::all(1.));
          obj
        })
      };
      @ $p {
        @{ Void }
      }
    };

    let mut wnd = TestWindow::new(widget);
    wnd.draw_frame();
    *son_writer.write() += 1;
    wnd.draw_frame();

    // If the parent pipe is not correctly collected, it may cause a panic when
    // refreshing the child widget of the pipe.
    *m_writer.write() += 1;
    wnd.draw_frame();
  }
}
