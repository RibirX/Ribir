use std::{
  cell::{Cell, RefCell, UnsafeCell},
  convert::Infallible,
  ops::RangeInclusive,
};

use ribir_algo::Sc;
use rxrust::ops::box_it::BoxOp;
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::{
  builtin_widgets::key::AnyKey,
  prelude::*,
  render_helper::{PureRender, RenderProxy},
  window::WindowId,
};

pub type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

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

  /// Chain more operations on the pipe value stream by applying the `f` on the
  /// final value stream when subscribe. This is a lazy operation, it will
  /// not execute the `f` until the pipe is be subscribed.
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
  /// - *priority*: defines the priority of the emitted values if it is a
  ///   Some-value.
  fn unzip(
    self, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>);

  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>);
}

/// A trait object type for `Pipe`, help to store a concrete `Pipe`
/// or just a value.
///
/// This type not implement `Pipe` trait to avoid boxing the `Pipe` twice and
/// has a better conversion from `Pipe` to `BoxPipe`.
///
/// Call `into_pipe` to convert it to a `Pipe` type.
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
    Self::Value: IntoWidget<'static, M>,
  {
    let f = move |ctx: &mut BuildCtx| {
      let info = DynWidgetsInfo::new(GenRange::Single(ctx.tree().root()));

      let priority = UpdatePriority::new_with_wnd(info.clone(), ctx.window().id());
      let (w, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(Box::new(priority)));

      let provider = Box::new(Queryable(info.clone()));
      let (w, _) = ctx.consume_root_with_provider(w.into_widget(), provider);

      w.on_build(|w, ctx| {
        info
          .borrow_mut()
          .set_gen_range(GenRange::Single(w));

        let pipe_node = PipeNode::share_capture(w, info, ctx);
        let c_pipe_node = pipe_node.clone();

        let tree = ctx.tree;
        let u = modifies.subscribe(move |(_, w)| {
          let info = pipe_node.dyn_info();
          let old = info.borrow().host_id();
          let mut ctx = BuildCtx::create(old, tree);

          let old_node = pipe_node.remove_old_data();
          let new = w.into_widget().build(&mut ctx);
          let tree = ctx.tree_mut();
          pipe_node.transplant_to_new(old_node, new, tree);

          query_outside_infos(new, &info, tree)
            .for_each(|info| info.borrow_mut().single_replace(old, new));

          update_key_status_single(old, new, tree);
          old.insert_after(new, tree);
          old.dispose_subtree(tree);
          new.on_mounted_subtree(tree);

          tree.mark_dirty(new);
        });
        c_pipe_node.own_subscription(u, ctx);
      })
    };
    f.into_widget()
  }

  fn build_multi<const M: usize>(self) -> Vec<Widget<'static>>
  where
    Self::Value: IntoIterator,
    <Self::Value as IntoIterator>::Item: IntoWidget<'static, M>,
  {
    let info = DynWidgetsInfo::new(GenRange::Multi(vec![]));

    let priority = UpdatePriority::new(info.clone());
    let (m, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(Box::new(priority.clone())));
    let mut iter = m.into_iter();

    let first = iter
      .next()
      .map_or_else(|| Void.into_widget(), IntoWidget::into_widget);

    let provider = Box::new(Queryable(info.clone()));
    let first = fn_widget! {
      let (w, _) = ctx!().consume_root_with_provider(first, provider);
      w
    };

    let info2 = info.clone();
    let first = first.into_widget().on_build(move |id, ctx| {
      match &mut info2.borrow_mut().gen_range {
        GenRange::Multi(m) => m.push(id),
        _ => unreachable!(),
      };

      priority.set_wnd(ctx.window().id());
      let pipe_node = PipeNode::share_capture(id, info2, ctx);
      let c_pipe_node = pipe_node.clone();

      let tree = ctx.tree;
      let u = modifies.subscribe(move |(_, m)| {
        let info = pipe_node.dyn_info();
        let old = match &info.borrow().gen_range {
          GenRange::Multi(m) => m.clone(),
          _ => unreachable!(),
        };

        let mut ctx = BuildCtx::create(old[0], tree);

        let old_node = pipe_node.remove_old_data();
        let mut new = vec![];
        for (idx, w) in m.into_iter().enumerate() {
          let id = w.into_widget().build(&mut ctx);
          new.push(id);
          set_pos_of_multi(id, idx, ctx.tree());
        }
        if new.is_empty() {
          new.push(Void.into_widget().build(&mut ctx));
        }

        let tree = ctx.tree_mut();
        pipe_node.transplant_to_new(old_node, new[0], tree);

        query_outside_infos(new[0], &info, tree)
          .for_each(|info| info.borrow_mut().multi_replace(&old, &new));

        update_key_state_multi(old.iter().copied(), new.iter().copied(), tree);

        new
          .iter()
          .rev()
          .for_each(|w| old[0].insert_after(*w, tree));
        old.iter().for_each(|id| id.dispose_subtree(tree));
        new.iter().for_each(|w| {
          w.on_mounted_subtree(tree);
          tree.mark_dirty(*w)
        });
      });

      c_pipe_node.own_subscription(u, ctx);
    });

    let mut widgets = vec![first];
    for (idx, w) in iter.enumerate() {
      let info = info.clone();
      let w = w.into_widget().on_build(move |id, ctx| {
        match &mut info.borrow_mut().gen_range {
          GenRange::Multi(m) => m.push(id),
          _ => unreachable!(),
        };

        let tree = ctx.tree_mut();
        if set_pos_of_multi(id, idx + 1, tree) {
          // We need to associate the parent information with the children pipe so that
          // when the child pipe is regenerated, it can update the parent pipe information
          // accordingly.
          id.attach_data(Box::new(Queryable(info.clone())), tree);
        }
      });

      widgets.push(w);
    }

    widgets
  }

  fn into_parent_widget<const M: usize>(self) -> Widget<'static>
  where
    Self: Sized,
    Self::Value: IntoWidget<'static, M>,
  {
    let f = move |ctx: &mut BuildCtx| {
      let root = ctx.tree().root();
      let info = DynWidgetsInfo::new(GenRange::ParentOnly(root..=root));

      let priority = UpdatePriority::new_with_wnd(info.clone(), ctx.window().id());
      let (w, modifies) = self.unzip(ModifyScope::FRAMEWORK, Some(Box::new(priority)));

      let provider = Box::new(Queryable(info.clone()));
      let (w, _) = ctx.consume_root_with_provider(w.into_widget(), provider);
      w.on_build(|p, ctx| {
        let pipe_node = PipeNode::share_capture(p, info.clone(), ctx);
        let tree = ctx.tree_mut();
        let leaf = p.single_leaf(tree);
        info
          .borrow_mut()
          .set_gen_range(GenRange::ParentOnly(p..=leaf));

        // We need to associate the parent information with the pipe of leaf widget,
        // when the leaf pipe is regenerated, it can update the parent pipe information
        // accordingly.
        if leaf.contain_type::<DynInfo>(tree) {
          leaf.attach_data(Box::new(Queryable(info)), tree);
        };

        let c_pipe_node = pipe_node.clone();
        let tree = ctx.tree;
        let u = modifies.subscribe(move |(_, w)| {
          let info = pipe_node.dyn_info();
          let (top, bottom) = match &info.borrow().gen_range {
            GenRange::ParentOnly(p) => p.clone().into_inner(),
            _ => unreachable!(),
          };

          let mut ctx = BuildCtx::create(top, tree);

          let old_node = pipe_node.remove_old_data();
          let p = w.into_widget().build(&mut ctx);
          let tree = ctx.tree_mut();
          pipe_node.transplant_to_new(old_node, p, tree);

          let new_rg = p..=p.single_leaf(tree);
          let children: SmallVec<[WidgetId; 1]> = bottom.children(tree).collect();
          for c in children {
            new_rg.end().append(c, tree);
          }

          query_outside_infos(p, &info, tree).for_each(|info| {
            info
              .borrow_mut()
              .single_range_replace(&(top..=bottom), &new_rg);
          });

          update_key_status_single(top, p, tree);
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

          tree.mark_dirty(p);
        });

        c_pipe_node.own_subscription(u, ctx);
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
    self, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let stream = self
      .0
      .filter(move |s| s.contains(scope))
      .map(|s| (s, s));

    let stream = if let Some(priority) = priority {
      stream
        .sample(AppCtx::frame_ticks().clone())
        .priority(priority)
        .box_it()
    } else {
      stream.box_it()
    };

    (ModifyScope::empty(), stream)
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, priority)
  }
}

impl<V: 'static> Pipe for Box<dyn Pipe<Value = V>> {
  type Value = V;

  #[inline]
  fn unzip(
    self, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.box_unzip(scope, priority)
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).box_unzip(scope, priority)
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
    self, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f, .. } = self;
    let (v, stream) = source.unzip(scope, priority);
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, priority)
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
    self, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, f, .. } = self;
    let (v, stream) = source.unzip(scope, priority);
    (v, f(stream))
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).unzip(scope, priority)
  }
}

/// A pipe that never changes, help to construct a pipe from a value.
struct ValuePipe<V>(V);

impl<V: 'static> Pipe for ValuePipe<V> {
  type Value = V;

  #[inline]
  fn unzip(
    self, _: ModifyScope, _: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (self.0, observable::empty().box_it())
  }

  #[inline]
  fn box_unzip(
    self: Box<Self>, scope: ModifyScope, priority: Option<Box<dyn Priority>>,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.unzip(scope, priority)
  }
}

impl<T: Pipe> InnerPipe for T {}

impl<V, S, F, const M: usize> IntoWidgetStrict<'static, M> for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: IntoWidget<'static, M>,
{
  fn into_widget_strict(self) -> Widget<'static> { self.build_single() }
}

impl<V, S, F, const M: usize> IntoWidgetStrict<'static, M> for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: IntoWidget<'static, M>,
{
  fn into_widget_strict(self) -> Widget<'static> { self.build_single() }
}

impl<const M: usize, V> IntoWidgetStrict<'static, M> for Box<dyn Pipe<Value = V>>
where
  V: IntoWidget<'static, M>,
{
  fn into_widget_strict(self) -> Widget<'static> { self.build_single() }
}

impl<V, S, F, const M: usize> IntoWidget<'static, M> for MapPipe<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: IntoWidget<'static, M>,
{
  fn into_widget(self) -> Widget<'static> { option_into_widget(self) }
}

impl<V, S, F, const M: usize> IntoWidget<'static, M> for FinalChain<Option<V>, S, F>
where
  Self: InnerPipe<Value = Option<V>>,
  V: IntoWidget<'static, M>,
{
  fn into_widget(self) -> Widget<'static> { option_into_widget(self) }
}

impl<const M: usize, V> IntoWidget<'static, M> for Box<dyn Pipe<Value = Option<V>>>
where
  V: IntoWidget<'static, M>,
{
  fn into_widget(self) -> Widget<'static> { option_into_widget(self) }
}

fn option_into_widget<const M: usize>(
  p: impl InnerPipe<Value = Option<impl IntoWidget<'static, M>>>,
) -> Widget<'static> {
  p.map(|w| move |_: &mut BuildCtx| w.map_or_else(|| Void.into_widget(), IntoWidget::into_widget))
    .build_single()
}

fn update_children_key_status(old: WidgetId, new: WidgetId, tree: &WidgetTree) {
  match (old.first_child(tree), old.last_child(tree), new.first_child(tree), new.last_child(tree)) {
    // old or new children is empty.
    (None, _, _, _) | (_, _, None, _) => {}
    (Some(_), None, _, _) | (_, _, Some(_), None) => {
      unreachable!("first child is some, but last child is none")
    }
    (Some(o_first), Some(o_last), Some(n_first), Some(n_last)) => {
      match (o_first == o_last, n_first == n_last) {
        (true, true) => update_key_status_single(o_first, n_first, tree),
        (true, false) => {
          if let Some(old_key) = o_first.query_ref::<Box<dyn AnyKey>>(tree) {
            let o_key = old_key.key();
            new.children(tree).any(|n| {
              if let Some(new_key) = n.query_ref::<Box<dyn AnyKey>>(tree) {
                let same_key = o_key == new_key.key();
                if same_key {
                  update_key_states(&**old_key, o_first, &**new_key, n, tree);
                }
                same_key
              } else {
                false
              }
            });
          }
        }
        (false, true) => {
          if let Some(new_key) = n_first.query_ref::<Box<dyn AnyKey>>(tree) {
            let n_key = new_key.key();
            old.children(tree).any(|o| {
              if let Some(old_key) = o.query_ref::<Box<dyn AnyKey>>(tree) {
                let same_key = old_key.key() == n_key;
                if same_key {
                  update_key_states(&**old_key, o, &**new_key, n_first, tree);
                }
                same_key
              } else {
                false
              }
            });
          }
        }
        (false, false) => update_key_state_multi(old.children(tree), new.children(tree), tree),
      }
    }
  }
}

fn update_key_status_single(old: WidgetId, new: WidgetId, tree: &WidgetTree) {
  if let Some(old_key) = old.query_ref::<Box<dyn AnyKey>>(tree) {
    if let Some(new_key) = new.query_ref::<Box<dyn AnyKey>>(tree) {
      if old_key.key() == new_key.key() {
        update_key_states(&**old_key, old, &**new_key, new, tree);
      }
    }
  }
}

fn update_key_state_multi(
  old: impl Iterator<Item = WidgetId>, new: impl Iterator<Item = WidgetId>, tree: &WidgetTree,
) {
  let mut old_key_list = ahash::HashMap::default();
  for o in old {
    if let Some(old_key) = o.query_ref::<Box<dyn AnyKey>>(tree) {
      old_key_list.insert(old_key.key(), o);
    }
  }

  if !old_key_list.is_empty() {
    for n in new {
      if let Some(new_key) = n.query_ref::<Box<dyn AnyKey>>(tree) {
        if let Some(o) = old_key_list.get(&new_key.key()).copied() {
          if let Some(old_key) = o.query_ref::<Box<dyn AnyKey>>(tree) {
            update_key_states(&**old_key, o, &**new_key, n, tree);
          }
        }
      }
    }
  }
}

fn update_key_states(
  old_key: &dyn AnyKey, old: WidgetId, new_key: &dyn AnyKey, new: WidgetId, tree: &WidgetTree,
) {
  new_key.record_prev_key_widget(old_key);
  old_key.record_next_key_widget(new_key);
  update_children_key_status(old, new, tree)
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
struct PipeNode(Sc<UnsafeCell<InnerPipeNode>>);

struct InnerPipeNode {
  data: Box<dyn RenderQueryable>,
  dyn_info: DynInfo,
}

#[derive(Debug)]
pub(crate) struct DynWidgetsInfo {
  multi_pos: usize,
  gen_range: GenRange,
}

pub(crate) type DynInfo = Sc<RefCell<DynWidgetsInfo>>;

#[derive(Debug)]
enum GenRange {
  Single(WidgetId),
  Multi(Vec<WidgetId>),
  ParentOnly(RangeInclusive<WidgetId>),
}

impl DynWidgetsInfo {
  fn new(range: GenRange) -> DynInfo {
    Sc::new(RefCell::new(DynWidgetsInfo { gen_range: range, multi_pos: 0 }))
  }

  fn set_gen_range(&mut self, range: GenRange) { self.gen_range = range; }

  pub(crate) fn single_replace(&mut self, old: WidgetId, new: WidgetId) {
    match &mut self.gen_range {
      GenRange::Single(id) => {
        assert_eq!(*id, old, "For single pipe node, the logic pipe child must be same `PipeNode`.");
        *id = new;
      }
      GenRange::Multi(m) => {
        if let Some(idx) = m.iter().position(|w| *w == old) {
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
        if let Some(idx) = m.iter().position(|w| *w == p) {
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

  fn multi_replace(&mut self, old: &[WidgetId], new: &[WidgetId]) {
    match &mut self.gen_range {
      GenRange::Single(_) => unreachable!("Single pipe node never have multi pipe child."),
      GenRange::Multi(m) => {
        if let Some(from) = m.iter().position(|w| &old[0] == w) {
          let to = m
            .iter()
            .position(|w| &old[old.len() - 1] == w)
            .expect("must include");
          m.splice(from..=to, new.iter().copied());
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

  fn set_pos_of_multi(&mut self, pos: usize) { self.multi_pos = pos; }

  fn pos_of_multi(&self) -> usize { self.multi_pos }
}

impl PipeNode {
  fn share_capture(id: WidgetId, dyn_info: DynInfo, ctx: &mut BuildCtx) -> Self {
    let mut pipe_node = None;

    id.wrap_node(ctx.tree_mut(), |r| {
      let inner_node = InnerPipeNode { data: r, dyn_info };
      let p = Self(Sc::new(UnsafeCell::new(inner_node)));
      pipe_node = Some(p.clone());
      Box::new(p)
    });

    // Safety: init before.
    unsafe { pipe_node.unwrap_unchecked() }
  }

  fn dyn_info(&self) -> DynInfo { self.as_ref().dyn_info.clone() }

  // Remove the old widget so that the new widget build logic cannot access it
  // anymore.
  fn remove_old_data(&self) -> Box<dyn RenderQueryable> {
    std::mem::replace(&mut self.as_mut().data, Box::new(PureRender(Void)))
  }

  fn transplant_to_new(
    &self, old_node: Box<dyn RenderQueryable>, new: WidgetId, tree: &mut WidgetTree,
  ) {
    let old = self.as_ref().dyn_info.borrow().host_id();
    let [old, new] = tree.get_many_mut(&[old, new]);
    std::mem::swap(&mut self.as_mut().data, new);

    std::mem::swap(old, new);
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
  fn own_subscription(self, u: impl Subscription + 'static, ctx: &mut BuildCtx) {
    let node = self.as_mut();
    let id = node.dyn_info.borrow().host_id();
    let tree = ctx.tree_mut();
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

fn set_pos_of_multi(w: WidgetId, pos: usize, tree: &WidgetTree) -> bool {
  w.query_all_iter::<DynInfo>(tree)
    .inspect(|info| info.borrow_mut().set_pos_of_multi(pos))
    .count()
    > 0
}
fn query_outside_infos<'l>(
  id: WidgetId, to: &'l DynInfo, tree: &'l WidgetTree,
) -> impl Iterator<Item = QueryRef<'l, DynInfo>> {
  let mut hit = false;
  id.query_all_iter::<DynInfo>(tree)
    .rev()
    .take_while(move |info| {
      if hit {
        false
      } else {
        hit = Sc::ptr_eq(info, to);
        true
      }
    })
}

fn pipe_priority_value(info: &DynInfo, wnd: &Window) -> i64 {
  let id = info.borrow().host_id();
  let tree = wnd.tree();
  let depth = id.ancestors(tree).count() as i64;
  let embed = query_outside_infos(id, info, tree).count() as i64;

  let pos = info.borrow_mut().pos_of_multi() as i64;
  depth << 60 | pos << 40 | embed
}

impl Query for PipeNode {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    let p = self.as_ref();
    p.data.query_all(type_id, out);
    if type_id == TypeId::of::<DynInfo>() {
      out.push(QueryHandle::new(&p.dyn_info))
    }
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    let p = self.as_ref();
    p.data
      .query(type_id)
      .or_else(|| (type_id == TypeId::of::<DynInfo>()).then(|| QueryHandle::new(&p.dyn_info)))
  }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.as_ref().data.query_write(type_id)
  }

  fn queryable(&self) -> bool { true }
}

impl RenderProxy for PipeNode {
  type Target<'r> = &'r dyn RenderQueryable
    where
      Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { &*self.as_ref().data }
}

#[derive(Clone)]
struct UpdatePriority {
  wnd_id: Sc<Cell<Option<WindowId>>>,
  dyn_info: DynInfo,
}

impl UpdatePriority {
  fn new_with_wnd(multi_info: DynInfo, wnd_id: WindowId) -> Self {
    let p = Self::new(multi_info);
    p.set_wnd(wnd_id);
    p
  }

  fn new(multi_info: DynInfo) -> Self { Self { wnd_id: <_>::default(), dyn_info: multi_info } }

  fn set_wnd(&self, wnd_id: WindowId) {
    assert!(self.wnd_id.get().is_none());
    self.wnd_id.set(Some(wnd_id));
  }
}

impl Priority for UpdatePriority {
  fn priority(&mut self) -> i64 {
    self
      .wnd_id
      .get()
      .and_then(|wnd_id| AppCtx::get_window(wnd_id))
      .map_or(-1, |wnd| pipe_priority_value(&self.dyn_info, &wnd))
  }

  fn queue(&mut self) -> Option<&PriorityTaskQueue> {
    self.wnd_id.get().and_then(|wnd_id| {
      AppCtx::get_window(wnd_id).map(|wnd| {
        let queue = wnd.priority_task_queue();
        // Safety: This trait is only used within this crate, and we can ensure that
        // the window is valid when utilizing the `PriorityTaskQueue`.
        unsafe { std::mem::transmute(queue) }
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use ribir_dev_helper::assert_layout_result_by_path;

  use crate::{
    builtin_widgets::key::{AnyKey, KeyChange},
    prelude::*,
    reset_test_env,
    test_helper::*,
  };

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_as_root() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_writer();
    let w = fn_widget! {
      let p = pipe! { MockBox { size: *$size }};
      @$p { @Void {} }
    };
    let wnd = TestWindow::new(w);
    let tree = wnd.tree_mut();
    tree.layout(Size::zero());
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
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
          let p = pipe! { MockBox { size: *$size }};
          @$p { @Void {} }
        }
      }
    };
    let wnd = TestWindow::new(w);
    let tree = wnd.tree_mut();
    tree.layout(Size::zero());
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
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
  fn attach_data_to_pipe_widget() {
    reset_test_env!();
    let trigger = Stateful::new(false);
    let c_trigger = trigger.clone_watcher();
    let w = fn_widget! {
      let p = pipe! {
        // just use to force update the widget, when trigger modified.
        $c_trigger;
        MockBox { size: Size::zero() }
      };
      @KeyWidget {
        key: 0,
        value: (),
        @ { p }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    {
      *trigger.write() = true;
    }
    wnd.draw_frame();
    let tree = wnd.tree();

    // the key should still in the root widget after pipe widget updated.
    assert!(
      tree
        .content_root()
        .contain_type::<Box<dyn AnyKey>>(tree)
    );
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
            v.into_iter().map(move |_| {
              @MockBox{
                size: Size::zero(),
                on_mounted: move |_| *$new_cnt.write() += 1,
                on_disposed: move |_| *$drop_cnt.write() += 1
              }
            })
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
      pipe!(*$p).map(move |_| {
        @MockBox {
          size: Size::zero(),
          on_mounted: move |_| *$w_mnt_cnt.write() +=1,
          @{
            pipe!(*$c).map(move |_| {
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
  fn pipe_widgets_with_key() {
    reset_test_env!();

    let v = Stateful::new(vec![(1, '1'), (2, '2'), (3, '3')]);
    let enter_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let update_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let leave_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let key_change: Stateful<KeyChange<char>> = Stateful::new(KeyChange::default());

    let c_v = v.clone_writer();
    let c_enter_list = enter_list.clone_writer();
    let c_update_list = update_list.clone_writer();
    let c_leave_list = leave_list.clone_writer();
    let c_key_change = key_change.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!($v.clone()).map(move |v| {
            v.into_iter().map(move |(i, c)| {
              let key = @KeyWidget { key: i, value: c };
              @$key {
                @MockBox {
                  size: Size::zero(),
                  on_mounted: move |_| {
                    if $key.is_enter() {
                      $c_enter_list.write().push($key.value);
                    }

                    if $key.is_changed() {
                      $c_update_list.write().push($key.value);
                      *$c_key_change.write() = $key.get_change();
                    }
                  },
                  on_disposed: move |_| if $key.is_leave() {
                    $c_leave_list.write().push($key.value);
                  }
                }
              }
            })
          })
        }
      }
    };

    // 1. 3 item enter
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    let expect_vec = ['1', '2', '3'];
    assert_eq!((*enter_list.read()).len(), 3);
    assert!(
      (*enter_list.read())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    enter_list.write().clear();

    // 2. add 1 item
    c_v.write().push((4, '4'));
    wnd.draw_frame();

    let expect_vec = ['4'];
    assert_eq!((*enter_list.read()).len(), 1);
    assert!(
      enter_list
        .read()
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    enter_list.write().clear();

    // 3. update the second item
    c_v.write()[1].1 = 'b';
    wnd.draw_frame();

    let expect_vec = [];
    assert_eq!((*enter_list.read()).len(), 0);
    assert!(
      (*enter_list.read())
        .iter()
        .all(|item| expect_vec.contains(item))
    );

    let expect_vec = ['b'];
    assert_eq!((*update_list.read()).len(), 1);
    assert!(
      (*update_list.read())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.read(), KeyChange(Some('2'), 'b'));
    update_list.write().clear();

    // 4. remove the second item
    c_v.write().remove(1);
    wnd.draw_frame();
    let expect_vec = vec!['b'];
    assert_eq!((*leave_list.read()), expect_vec);
    assert_eq!((*leave_list.read()).len(), 1);
    assert!(
      leave_list
        .read()
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    leave_list.write().clear();

    // 5. update the first item
    c_v.write()[0].1 = 'a';
    wnd.draw_frame();

    assert_eq!((*enter_list.read()).len(), 0);

    let expect_vec = ['a'];
    assert_eq!((*update_list.read()).len(), 1);
    assert!(
      (*update_list.read())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.read(), KeyChange(Some('1'), 'a'));
    update_list.write().clear();
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

    fn build(task: Stateful<Task>) -> impl IntoWidget<'static, FN> {
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
          $c_tasks.iter().map(|t| build(t.clone_writer())).collect::<Vec<_>>()
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
        @ { pipe!(*$child).map(move |_| {
          @MockMulti {
            keep_alive: pipe!(!*$child_destroy_until),
            @ { pipe!(*$grandson).map(move |_| {
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
      let w = pipe!(*$v).map(move |_| Void.into_widget());
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
            (0..v).map(move |_| {
              pipe!(*$child_size).map(move |size| @MockBox { size })
            })
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(1., 1.), });

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(2., 1.), });

    *c_box_count.write() = 2;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(4., 1.), });
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
            (0..v).map(move |_| {
              let pipe_parent = pipe!(*$child_size).map(move |size| @MockBox { size });
              @$pipe_parent { @Void {} }
            })
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(1., 1.), });

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(2., 1.), });

    *c_box_count.write() = 2;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(4., 1.), });
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
          pipe!(*$pipe_trigger).map(move |w| {
            pipe!(*$inner_pipe_trigger)
              .map(move |h| @MockBox { size: Size::new(w as f32, h as f32) })
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 0.), });

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 1.), });

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(1., 1.), });
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
          pipe!(*$outer).map(move |w| {
            let pipe_parent = pipe!(*$inner)
              .map(move |h| @MockBox { size: Size::new(w as f32, h as f32) });
            @$pipe_parent { @Void {} }
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 0.), });

    println!("Inner pipe update:");
    *w_inner.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 1.), });

    println!("Outter pipe update:");
    *w_outer.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(1., 1.), });
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
          let p = pipe!(*$pipe_trigger).map(move |w| {
            pipe!(*$inner_pipe_trigger)
              .map(move |h| @MockBox { size: Size::new(w as f32, h as f32) })
          });

          @$p { @Void {} }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 0.), });

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(0., 1.), });

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    assert_layout_result_by_path!(wnd, { path = [0], size == Size::new(1., 1.), });
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
      pipe!($parent;).map(move |_| {
        pipe!($child;).map(move |_| {
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
          pipe!(*$r1).map(move |_|{
            pipe!(*$r2)
              .map(move |r| {
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
            $w.silent().push(0);
            (0..10).map(move |idx| {
              pipe!($w;).map(move |_| {
                $w.silent().push(idx + 1);
                @MockBox {
                  size: Size::new(1., 1.),
                }
              })
            })
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
          pipe!($m_watcher;).map(move |_| [
            @ { Void.into_widget() },
            @ { pipe!($son_watcher;).map(|_| Void).into_widget() }
          ])
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
        pipe!($m_watcher;).map(move |_| {
          // margin is static, but its child MockBox is a pipe.
          let p = pipe!($son_watcher;).map(|_| MockBox { size: Size::zero() });
          @$p { margin: EdgeInsets::all(1.) }
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
