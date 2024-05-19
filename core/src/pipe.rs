use std::{
  cell::{Cell, RefCell, UnsafeCell},
  convert::Infallible,
  ops::{Range, RangeInclusive},
};

use ribir_algo::Sc;
use rxrust::ops::box_it::BoxOp;

use crate::{
  builtin_widgets::key::AnyKey,
  prelude::*,
  render_helper::{RenderProxy, RenderTarget},
  ticker::FrameMsg,
};

type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

/// A trait for a value that can be subscribed its continuous modifies.
pub trait Pipe {
  type Value;
  /// Unzip the `Pipe` into its inner value and the changes stream of the
  /// value.
  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>);

  /// unzip the pipe value stream with a tick sample, and give a priority value
  /// to the stream to determine the priority of the downstream to be notified.
  /// This method only for build widget use.
  fn tick_unzip(
    self, prior_fn: impl FnMut() -> i64 + 'static, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>)
  where
    Self: Sized;

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>);

  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>);

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
    FinalChain { source: self, f }
  }
}

/// A trait object type for `Pipe`, help to store a concrete `Pipe`
/// or just a value.
///
/// This type not implement `Pipe` trait to avoid boxing the `Pipe` twice and
/// has a better conversion from `Pipe` to `BoxPipe`.
///
/// Call `into_pipe` to convert it to a `Pipe` type.
pub struct BoxPipe<V>(Box<dyn Pipe<Value = V>>);

pub struct MapPipe<V, S: Pipe, F: FnMut(S::Value) -> V> {
  source: S,
  f: F,
}

pub struct FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V>,
{
  source: S,
  f: F,
}

impl<V: 'static> BoxPipe<V> {
  #[inline]
  pub fn value(v: V) -> Self { Self(Box::new(ValuePipe(v))) }

  #[inline]
  pub fn pipe(p: Box<dyn Pipe<Value = V>>) -> Self { Self(p) }

  #[inline]
  pub fn into_pipe(self) -> Box<dyn Pipe<Value = V>> { self.0 }
}

pub(crate) trait InnerPipe: Pipe {
  fn build_single(
    self, ctx: &BuildCtx, build: impl Fn(Self::Value, &BuildCtx) -> Widget + 'static,
  ) -> Widget
  where
    Self: Sized,
    Self::Value: 'static,
  {
    let info =
      Sc::new(Cell::new(SinglePipeInfo { gen_id: ctx.tree.borrow().root(), multi_pos: 0 }));
    let info2 = info.clone();
    let handle = ctx.handle();
    let (w, modifies) = self.tick_unzip(move || pipe_priority_value(&info2, handle), ctx);
    let w = build(w, ctx);
    info.set(SinglePipeInfo { gen_id: w.id(), multi_pos: 0 });

    let pipe_node = PipeNode::share_capture(w.id(), Box::new(info.clone()), ctx);
    let c_pipe_node = pipe_node.clone();

    let u = modifies.subscribe(move |(_, w)| {
      handle.with_ctx(|ctx| {
        let id = info.host_id();

        let new_id = build(w, ctx).consume();

        query_info_outside_until(id, &info, ctx, |info| info.single_replace(id, new_id));
        pipe_node.primary_transplant(id, new_id, ctx);

        update_key_status_single(id, new_id, ctx);

        ctx.insert_after(id, new_id);
        ctx.dispose_subtree(id);
        ctx.on_subtree_mounted(new_id);

        ctx.mark_dirty(new_id);
      });
    });
    c_pipe_node.own_subscription(u, ctx);
    w
  }

  fn build_multi(
    self, vec: &mut Vec<Widget>,
    f: impl Fn(<Self::Value as IntoIterator>::Item, &BuildCtx) -> Widget + 'static, ctx: &BuildCtx,
  ) where
    Self::Value: IntoIterator,
    Self: Sized,
  {
    let build_multi = move |widgets: Self::Value, ctx: &BuildCtx| {
      let mut widgets = widgets
        .into_iter()
        .map(|w| f(w, ctx))
        .collect::<Vec<_>>();
      if widgets.is_empty() {
        widgets.push(Void.build(ctx));
      }
      widgets
    };

    let info = Sc::new(RefCell::new(MultiPipeInfo { widgets: vec![], multi_pos: 0 }));
    let info2 = info.clone();
    let handle = ctx.handle();
    let (m, modifies) = self.tick_unzip(move || pipe_priority_value(&info2, handle), ctx);

    let widgets = build_multi(m, ctx);
    let pipe_node = PipeNode::share_capture(widgets[0].id(), Box::new(info.clone()), ctx);
    let ids = widgets.iter().map(|w| w.id()).collect::<Vec<_>>();
    set_pos_of_multi(&ids, ctx);
    info.borrow_mut().widgets = ids;

    vec.extend(widgets);

    let c_pipe_node = pipe_node.clone();
    let u = modifies.subscribe(move |(_, m)| {
      handle.with_ctx(|ctx| {
        let old = info.borrow().widgets.clone();

        let new = build_multi(m, ctx)
          .into_iter()
          .map(Widget::consume)
          .collect::<Vec<_>>();

        set_pos_of_multi(&new, ctx);
        query_info_outside_until(old[0], &info, ctx, |info| info.multi_replace(&old, &new));
        pipe_node.primary_transplant(old[0], new[0], ctx);

        update_key_state_multi(old.iter().copied(), new.iter().copied(), ctx);

        new
          .iter()
          .rev()
          .for_each(|w| ctx.insert_after(old[0], *w));
        old.iter().for_each(|id| ctx.dispose_subtree(*id));
        new.iter().for_each(|w| {
          ctx.on_subtree_mounted(*w);
          ctx.mark_dirty(*w)
        });
      });
    });
    c_pipe_node.own_subscription(u, ctx);
  }

  fn only_parent_build(
    self, ctx: &BuildCtx, compose_child: impl FnOnce(Self::Value) -> (Widget, WidgetId),
    transplant: impl Fn(Self::Value, WidgetId, &BuildCtx) -> WidgetId + 'static,
  ) -> Widget
  where
    Self: Sized,
  {
    let root = ctx.tree.borrow().root();
    let info = Sc::new(RefCell::new(SingleParentPipeInfo { range: root..=root, multi_pos: 0 }));
    let info2 = info.clone();
    let handle = ctx.handle();
    let (v, modifies) = self.tick_unzip(move || pipe_priority_value(&info2, handle), ctx);
    let (p, child) = compose_child(v);
    let pipe_node = PipeNode::share_capture(p.id(), Box::new(info.clone()), ctx);
    let range = half_to_close_interval(p.id()..child, ctx);
    info.borrow_mut().range = range;

    let c_pipe_node = pipe_node.clone();

    let u = modifies.subscribe(move |(_, w)| {
      handle.with_ctx(|ctx| {
        let (top, bottom) = info.borrow().range.clone().into_inner();

        let first_child = bottom
          .first_child(&ctx.tree.borrow().arena)
          .unwrap();
        let p = transplant(w, bottom, ctx);
        let new_rg = half_to_close_interval(p..first_child, ctx);

        query_info_outside_until(top, &info, ctx, |info| {
          info.single_range_replace(&(top..=bottom), &new_rg);
        });
        pipe_node.primary_transplant(top, p, ctx);

        update_key_status_single(top, p, ctx);

        ctx.insert_after(top, p);
        ctx.dispose_subtree(top);
        for w in first_child.ancestors(&ctx.tree.borrow().arena) {
          ctx.on_widget_mounted(p);
          if w == p {
            break;
          }
        }

        ctx.mark_dirty(p);
      });
    });
    c_pipe_node.own_subscription(u, ctx);
    p
  }
}

impl<S: Pipe, V, F: FnMut(S::Value) -> V + 'static> MapPipe<V, S, F> {
  #[inline]
  pub fn new(source: S, f: F) -> Self { Self { source, f } }
}

pub struct ModifiesPipe(BoxOp<'static, ModifyScope, Infallible>);
impl ModifiesPipe {
  #[inline]
  pub fn new(modifies: BoxOp<'static, ModifyScope, Infallible>) -> Self { Self(modifies) }
}

impl Pipe for ModifiesPipe {
  type Value = ModifyScope;

  #[inline]
  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) {
    (ModifyScope::empty(), ObservableExt::map(self.0, |s| (s, s)).box_it())
  }

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }

  fn tick_unzip(
    self, prior_fn: impl FnMut() -> i64 + 'static, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let stream = self
      .0
      .filter(|s| s.contains(ModifyScope::FRAMEWORK))
      .sample(
        ctx
          .window()
          .frame_tick_stream()
          .filter(|f| matches!(f, FrameMsg::NewFrame(_))),
      )
      .prior_by(prior_fn, ctx.window().priority_task_queue().clone())
      .map(|s| (s, s))
      .box_it();

    (ModifyScope::empty(), stream)
  }

  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).tick_unzip(prior_fn, ctx)
  }
}

impl InnerPipe for ModifiesPipe {}

impl<V> Pipe for Box<dyn Pipe<Value = V>> {
  type Value = V;

  #[inline]
  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) { self.box_unzip() }

  #[inline]
  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).box_unzip() }

  #[inline]
  fn tick_unzip(
    self, prior_fn: impl FnMut() -> i64 + 'static, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.box_tick_unzip(Box::new(prior_fn), ctx)
  }

  #[inline]
  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).box_tick_unzip(prior_fn, ctx)
  }
}

impl<V> InnerPipe for Box<dyn Pipe<Value = V>> {}

impl<V: 'static, S: Pipe, F> Pipe for MapPipe<V, S, F>
where
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
  type Value = V;

  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f } = self;
    let (v, stream) = source.unzip();
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }

  #[inline]
  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }

  fn tick_unzip(
    self, prior_fn: impl FnMut() -> i64 + 'static, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f } = self;
    let (v, stream) = source.tick_unzip(prior_fn, ctx);
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }

  #[inline]
  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).tick_unzip(prior_fn, ctx)
  }
}

impl<V: 'static, S: InnerPipe, F> InnerPipe for MapPipe<V, S, F>
where
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
}

impl<V, S, F> Pipe for FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
{
  type Value = V;

  fn unzip(self) -> (V, ValueStream<V>) {
    let Self { source, f } = self;
    let (v, stream) = source.unzip();
    (v, f(stream))
  }

  #[inline]
  fn box_unzip(self: Box<Self>) -> (V, ValueStream<V>) { (*self).unzip() }

  fn tick_unzip(
    self, prior_fn: impl FnMut() -> i64 + 'static, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, f } = self;
    let (v, stream) = source.tick_unzip(prior_fn, ctx);
    (v, f(stream))
  }

  #[inline]
  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).tick_unzip(prior_fn, ctx)
  }
}

impl<V, S, F> InnerPipe for FinalChain<V, S, F>
where
  S: InnerPipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
{
}

/// A pipe that never changes, help to construct a pipe from a value.
struct ValuePipe<V>(V);

impl<V> Pipe for ValuePipe<V> {
  type Value = V;

  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) {
    (self.0, observable::empty().box_it())
  }

  #[inline]
  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }

  fn tick_unzip(
    self, _: impl FnMut() -> i64 + 'static, _: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    self.unzip()
  }

  #[inline]
  fn box_tick_unzip(
    self: Box<Self>, prior_fn: Box<dyn FnMut() -> i64>, ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    (*self).tick_unzip(prior_fn, ctx)
  }
}

crate::widget::multi_build_replace_impl! {
  impl<V, S, F> {#} for MapPipe<V, S, F>
  where
    V: {#} + 'static,
    S: InnerPipe,
    S::Value: 'static,
    F: FnMut(S::Value) -> V + 'static,
  {
    fn build(self, ctx: &BuildCtx) -> Widget {
      self.build_single(ctx, |w, ctx| w.build(ctx))
    }
  }

  impl<V, S, F> {#} for FinalChain<V, S, F>
  where
    V: {#} + 'static,
    S: InnerPipe<Value = V>,
    F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
  {
    fn build(self, ctx: &BuildCtx) -> Widget {
      self.build_single(ctx, |w, ctx| w.build(ctx))
    }
  }

  impl<V: {#} + 'static> {#} for Box<dyn Pipe<Value = V>>
  {
    fn build(self, ctx: &BuildCtx) -> Widget {
      self.build_single(ctx, |w, ctx| w.build(ctx))
    }
  }
}

impl<S, F> WidgetBuilder for MapPipe<Widget, S, F>
where
  S: InnerPipe,
  S::Value: 'static,
  F: FnMut(S::Value) -> Widget + 'static,
{
  fn build(self, ctx: &BuildCtx) -> Widget { self.build_single(ctx, |w, ctx| w.build(ctx)) }
}

impl<S, F> WidgetBuilder for FinalChain<Widget, S, F>
where
  S: InnerPipe<Value = Widget>,
  F: FnOnce(ValueStream<Widget>) -> ValueStream<Widget> + 'static,
{
  fn build(self, ctx: &BuildCtx) -> Widget { self.build_single(ctx, |w, ctx| w.build(ctx)) }
}

impl WidgetBuilder for Box<dyn Pipe<Value = Widget>> {
  fn build(self, ctx: &BuildCtx) -> Widget { self.build_single(ctx, |w, ctx| w.build(ctx)) }
}

macro_rules! pipe_option_to_widget {
  ($name:ident, $ctx:ident) => {
    $name
      .map(|w| {
        move |ctx: &BuildCtx| {
          if let Some(w) = w { w.build(ctx) } else { Void.build(ctx) }
        }
      })
      .build($ctx)
  };
}

pub(crate) use pipe_option_to_widget;

macro_rules! single_parent_impl {
  () => {
    fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
      self.only_parent_build(
        ctx,
        move |p| {
          let c = child.id();
          let p = p.compose_child(child, ctx);
          (p, c)
        },
        |new_p, old_p, ctx| {
          let child = old_p
            .single_child(&ctx.tree.borrow().arena)
            .unwrap();
          let child = Widget::from_id(child, ctx);
          new_p.compose_child(child, ctx).consume()
        },
      )
    }
  };
}

impl<V, S, F> SingleParent for MapPipe<V, S, F>
where
  S: InnerPipe,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
  single_parent_impl!();
}

impl<V, S, F> SingleParent for FinalChain<V, S, F>
where
  S: InnerPipe<Value = V>,
  V: SingleParent + RenderBuilder + 'static,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
{
  single_parent_impl!();
}

impl<V: SingleParent + RenderBuilder + 'static> SingleParent for Box<dyn Pipe<Value = V>> {
  single_parent_impl!();
}

macro_rules! multi_parent_impl {
  () => {
    fn compose_children(
      self, mut children: impl Iterator<Item = Widget>, ctx: &BuildCtx,
    ) -> Widget {
      // if children is empty, we can let the pipe parent as the whole subtree.
      let first_child = children.next();
      if let Some(first_child) = first_child {
        self.only_parent_build(
          ctx,
          move |p| {
            let child = first_child.id();
            let p = p.compose_children(children.chain(std::iter::once(first_child)), ctx);
            (p, child)
          },
          move |new_p, old_p, ctx| {
            // Safety: we escape the borrow of arena, but we only access the children of the
            // `old_p` and we know `compose_children` will not modifies the children of
            // `old_p`.
            let arena = unsafe { &(*ctx.tree.as_ptr()).arena };
            let children = old_p
              .children(arena)
              .map(|id| Widget::from_id(id, ctx));
            new_p
              .compose_children(children.into_iter(), ctx)
              .consume()
          },
        )
      } else {
        self.build(ctx)
      }
    }
  };
}

impl<V, S, F> MultiParent for MapPipe<V, S, F>
where
  S: InnerPipe,
  V: MultiParent + RenderBuilder + 'static,
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
  multi_parent_impl!();
}

impl<V, S, F> MultiParent for FinalChain<V, S, F>
where
  V: MultiParent + RenderBuilder + 'static,
  S: InnerPipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
{
  multi_parent_impl!();
}

impl<V: MultiParent + RenderBuilder + 'static> MultiParent for Box<dyn Pipe<Value = V>> {
  multi_parent_impl!();
}

macro_rules! option_single_parent_impl {
  () => {
    fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
      let handle = ctx.handle();
      Pipe::map(self, move |p| {
        handle
          .with_ctx(|ctx| {
            if let Some(p) = p {
              BoxedSingleChild::from_id(p.build(ctx))
            } else {
              BoxedSingleChild::new(Void, ctx)
            }
          })
          .expect("Context not available")
      })
      .compose_child(child, ctx)
    }
  };
}

impl<V, S, F> SingleParent for MapPipe<Option<V>, S, F>
where
  S: InnerPipe,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
  F: FnMut(S::Value) -> Option<V> + 'static,
{
  option_single_parent_impl!();
}

impl<V, S, F> SingleParent for FinalChain<Option<V>, S, F>
where
  S: InnerPipe<Value = Option<V>>,
  V: SingleParent + RenderBuilder + 'static,
  F: FnOnce(ValueStream<Option<V>>) -> ValueStream<Option<V>> + 'static,
{
  option_single_parent_impl!();
}

impl<V> SingleParent for Box<dyn Pipe<Value = Option<V>>>
where
  V: SingleParent + RenderBuilder + 'static,
{
  option_single_parent_impl!();
}

fn half_to_close_interval(rg: Range<WidgetId>, ctx: &BuildCtx) -> RangeInclusive<WidgetId> {
  rg.start..=rg.end.parent(&ctx.tree.borrow().arena).unwrap()
}

fn update_children_key_status(old: WidgetId, new: WidgetId, ctx: &BuildCtx) {
  let tree = &ctx.tree.borrow().arena;

  match (old.first_child(tree), old.last_child(tree), new.first_child(tree), new.last_child(tree)) {
    // old or new children is empty.
    (None, _, _, _) | (_, _, None, _) => {}
    (Some(_), None, _, _) | (_, _, Some(_), None) => {
      unreachable!("first child is some, but last child is none")
    }
    (Some(o_first), Some(o_last), Some(n_first), Some(n_last)) => {
      match (o_first == o_last, n_first == n_last) {
        (true, true) => update_key_status_single(o_first, n_first, ctx),
        (true, false) => {
          inspect_key(o_first, ctx, |old_key| {
            let o_key = old_key.key();
            new.children(tree).any(|n| {
              inspect_key(n, ctx, |new_key| {
                let same_key = o_key == new_key.key();
                if same_key {
                  update_key_states(old_key, o_first, new_key, n, ctx);
                }
                same_key
              })
              .unwrap_or(false)
            });
          });
        }
        (false, true) => {
          inspect_key(n_first, ctx, |new_key| {
            let n_key = new_key.key();
            old.children(tree).any(|o| {
              inspect_key(o, ctx, |old_key| {
                let same_key = old_key.key() == n_key;
                if same_key {
                  update_key_states(old_key, o, new_key, n_first, ctx);
                }
                same_key
              })
              .unwrap_or(false)
            })
          });
        }
        (false, false) => update_key_state_multi(old.children(tree), new.children(tree), ctx),
      }
    }
  }
}

fn update_key_status_single(old: WidgetId, new: WidgetId, ctx: &BuildCtx) {
  inspect_key(old, ctx, |old_key| {
    inspect_key(new, ctx, |new_key| {
      if old_key.key() == new_key.key() {
        update_key_states(old_key, old, new_key, new, ctx)
      }
    })
  });
}

fn update_key_state_multi(
  old: impl Iterator<Item = WidgetId>, new: impl Iterator<Item = WidgetId>, ctx: &BuildCtx,
) {
  let mut old_key_list = ahash::HashMap::default();
  for o in old {
    inspect_key(o, ctx, |old_key: &dyn AnyKey| {
      old_key_list.insert(old_key.key(), o);
    });
  }

  if !old_key_list.is_empty() {
    for n in new {
      inspect_key(n, ctx, |new_key| {
        if let Some(o) = old_key_list.get(&new_key.key()).copied() {
          inspect_key(o, ctx, |old_key| update_key_states(old_key, o, new_key, n, ctx));
        }
      });
    }
  }
}

fn inspect_key<R>(id: WidgetId, ctx: &BuildCtx, mut cb: impl FnMut(&dyn AnyKey) -> R) -> Option<R> {
  ctx
    .assert_get(id)
    .query_most_outside::<Box<dyn AnyKey>, _>(|key_widget| cb(key_widget.deref()))
}

fn update_key_states(
  old_key: &dyn AnyKey, old: WidgetId, new_key: &dyn AnyKey, new: WidgetId, ctx: &BuildCtx,
) {
  new_key.record_prev_key_widget(old_key);
  old_key.record_next_key_widget(new_key);
  update_children_key_status(old, new, ctx)
}

impl<S: Pipe, V: SingleChild, F: FnMut(S::Value) -> V + 'static> SingleChild for MapPipe<V, S, F> {}
impl<S, V, F> MultiChild for FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  V: MultiChild,
  F: FnOnce(ValueStream<V>) -> ValueStream<V>,
{
}
impl<S: Pipe, V: SingleChild, F: FnMut(S::Value) -> Option<V> + 'static> SingleChild
  for MapPipe<Option<V>, S, F>
{
}
impl<V: SingleChild> SingleChild for Box<dyn Pipe<Value = V>> {}
impl<V: MultiChild> MultiChild for Box<dyn Pipe<Value = V>> {}

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
  data: Box<dyn Render>,
  dyn_info: Box<dyn DynWidgetInfo>,
}
trait DynWidgetInfo: Any {
  fn single_replace(&self, old: WidgetId, new: WidgetId);
  fn single_range_replace(&self, old: &RangeInclusive<WidgetId>, new: &RangeInclusive<WidgetId>);
  fn multi_replace(&self, old: &[WidgetId], new: &[WidgetId]);
  fn as_any(&self) -> &dyn Any;
  fn host_id(&self) -> WidgetId;
  // return the position of the pipe node if it generated by a multi pipe.
  fn pos_of_multi(&self) -> usize;
  fn set_pos_of_multi(&self, pos: usize);
}

type DynInfo = Box<dyn DynWidgetInfo>;

#[derive(Clone, Copy)]
struct SinglePipeInfo {
  gen_id: WidgetId,
  multi_pos: usize,
}

impl DynWidgetInfo for Sc<Cell<SinglePipeInfo>> {
  fn single_replace(&self, old: WidgetId, new: WidgetId) {
    let mut v = self.get();
    assert_eq!(
      v.gen_id, old,
      "For single pipe node, the logic pipe child must be same `PipeNode`."
    );
    v.gen_id = new;
    self.set(v);
  }

  fn single_range_replace(&self, old: &RangeInclusive<WidgetId>, new: &RangeInclusive<WidgetId>) {
    let mut v = self.get();
    if *old.start() == v.gen_id {
      v.gen_id = *new.start();
      self.set(v)
    }
  }

  fn multi_replace(&self, _: &[WidgetId], _: &[WidgetId]) {
    unreachable!("Single pipe node never have multi pipe child.");
  }

  fn as_any(&self) -> &dyn Any { self }

  fn host_id(&self) -> WidgetId { self.get().gen_id }

  fn pos_of_multi(&self) -> usize { self.get().multi_pos }

  fn set_pos_of_multi(&self, pos: usize) {
    let mut v = self.get();
    v.multi_pos = pos;
    self.set(v);
  }
}

#[derive(Clone)]
struct SingleParentPipeInfo {
  range: RangeInclusive<WidgetId>,
  multi_pos: usize,
}

impl DynWidgetInfo for Sc<RefCell<SingleParentPipeInfo>> {
  fn single_replace(&self, old: WidgetId, new: WidgetId) {
    let mut v = self.borrow_mut();
    if v.range.start() == &old {
      v.range = new..=*v.range.end();
    }
    if v.range.end() == &old {
      v.range = *v.range.start()..=new;
    }
  }

  fn single_range_replace(&self, old: &RangeInclusive<WidgetId>, new: &RangeInclusive<WidgetId>) {
    let mut this = self.borrow_mut();
    if this.range.start() == old.start() {
      this.range = *new.start()..=*this.range.end();
    }
    if this.range.end() == old.end() {
      this.range = *this.range.start()..=*new.end();
    }
  }

  fn multi_replace(&self, _: &[WidgetId], _: &[WidgetId]) {
    unreachable!("Single parent node never have multi pipe child.");
  }

  fn as_any(&self) -> &dyn Any { self }

  fn host_id(&self) -> WidgetId { *self.borrow().range.start() }

  fn pos_of_multi(&self) -> usize { self.borrow().multi_pos }

  fn set_pos_of_multi(&self, pos: usize) {
    let mut v = self.borrow_mut();
    v.multi_pos = pos;
  }
}

struct MultiPipeInfo {
  widgets: Vec<WidgetId>,
  multi_pos: usize,
}
impl DynWidgetInfo for Sc<RefCell<MultiPipeInfo>> {
  fn single_replace(&self, old: WidgetId, new: WidgetId) {
    let mut this = self.borrow_mut();
    if let Some(idx) = this.widgets.iter().position(|w| *w == old) {
      this.widgets[idx] = new;
    }
  }

  fn single_range_replace(&self, old: &RangeInclusive<WidgetId>, new: &RangeInclusive<WidgetId>) {
    let mut this = self.borrow_mut();
    let p = *old.start();
    if let Some(idx) = this.widgets.iter().position(|w| *w == p) {
      this.widgets[idx] = *new.start();
    }
  }

  fn multi_replace(&self, old: &[WidgetId], new: &[WidgetId]) {
    let mut this = self.borrow_mut();
    if let Some(from) = this.widgets.iter().position(|w| &old[0] == w) {
      let to = this
        .widgets
        .iter()
        .position(|w| &old[old.len() - 1] == w)
        .expect("must include");
      this
        .widgets
        .splice(from..=to, new.iter().copied());
    }
  }

  fn as_any(&self) -> &dyn Any { self }

  fn host_id(&self) -> WidgetId { *self.borrow().widgets.first().unwrap() }

  fn pos_of_multi(&self) -> usize { self.borrow().multi_pos }

  fn set_pos_of_multi(&self, pos: usize) {
    let mut v = self.borrow_mut();
    v.multi_pos = pos;
  }
}

impl PipeNode {
  fn share_capture(id: WidgetId, dyn_info: Box<dyn DynWidgetInfo>, ctx: &BuildCtx) -> Self {
    let tree = &mut ctx.tree.borrow_mut().arena;
    let mut pipe_node = None;

    id.wrap_node(tree, |r| {
      let inner_node = InnerPipeNode { data: r, dyn_info };
      let p = Self(Sc::new(UnsafeCell::new(inner_node)));
      pipe_node = Some(p.clone());
      Box::new(RenderProxy::new(p))
    });

    // Safety: init before.
    unsafe { pipe_node.unwrap_unchecked() }
  }

  // update the primary `PipeNode`.
  fn primary_transplant(&self, old: WidgetId, new: WidgetId, ctx: &BuildCtx) {
    let mut tree = ctx.tree.borrow_mut();
    let [old_node, new_node] = tree.get_many_mut(&[old, new]);
    std::mem::swap(&mut self.as_mut().data, new_node);
    std::mem::swap(old_node, new_node);
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
  fn own_subscription(self, u: impl Subscription + 'static, ctx: &BuildCtx) {
    let node = self.as_mut();
    let id = node.dyn_info.host_id();
    let tree = &mut ctx.tree.borrow_mut().arena;
    // if the subscription is closed, we can cancel and unwrap the `PipeNode`
    // immediately.
    if u.is_closed() {
      let v = std::mem::replace(&mut self.as_mut().data, Box::new(Void));
      *id.get_node_mut(tree).unwrap() = v;
    } else {
      id.attach_anonymous_data(u.unsubscribe_when_dropped(), tree)
    }
  }
}

fn set_pos_of_multi(widgets: &[WidgetId], ctx: &BuildCtx) {
  let arena = &ctx.tree.borrow().arena;
  widgets.iter().enumerate().for_each(|(pos, wid)| {
    wid
      .assert_get(arena)
      .query_type_outside_first(|info: &DynInfo| {
        info.set_pos_of_multi(pos);
        true
      });
  });
}

fn query_info_outside_until<T: Any>(
  id: WidgetId, to: &Sc<T>, ctx: &BuildCtx, mut cb: impl FnMut(&DynInfo),
) {
  id.assert_get(&ctx.tree.borrow().arena)
    .query_type_outside_first(|info: &DynInfo| {
      cb(info);

      info
        .as_any()
        .downcast_ref::<Sc<T>>()
        .map_or(true, |info| !Sc::ptr_eq(info, to))
    });
}

fn pipe_priority_value<T: Any>(info: &Sc<T>, handle: BuildCtxHandle) -> i64
where
  Sc<T>: DynWidgetInfo,
{
  handle
    .with_ctx(|ctx| {
      let id = info.host_id();
      let depth = id.ancestors(&ctx.tree.borrow().arena).count() as i64;
      let mut embed = 0;
      query_info_outside_until(id, info, ctx, |_| {
        embed += 1;
      });
      let pos = info.pos_of_multi() as i64;
      depth << 60 | pos << 40 | embed
    })
    .unwrap_or(-1)
}

impl Query for PipeNode {
  fn query_inside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    let p = self.as_ref();
    p.data.query_inside_first(type_id, callback) && p.dyn_info.query_inside_first(type_id, callback)
  }

  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    let p = self.as_ref();
    p.dyn_info.query_outside_first(type_id, callback)
      && p.data.query_outside_first(type_id, callback)
  }
}

impl Query for Box<dyn DynWidgetInfo> {
  crate::widget::impl_query_self_only!();
}

impl RenderTarget for PipeNode {
  type Target = dyn Render;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.as_ref().data) }
}

#[cfg(test)]
mod tests {
  use std::{
    cell::{Cell, Ref},
    rc::Rc,
  };

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
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::zero());
    let ids = tree
      .content_root()
      .descendants(&tree.arena)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree
      .content_root()
      .descendants(&tree.arena)
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
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::zero());
    let ids = tree
      .content_root()
      .descendants(&tree.arena)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree
      .content_root()
      .descendants(&tree.arena)
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
    let tree = wnd.widget_tree.borrow();

    // the key should still in the root widget after pipe widget updated.
    assert!(
      tree
        .content_root()
        .assert_get(&tree.arena)
        .contain_type::<Box<dyn AnyKey>>()
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
    let p_trigger = Stateful::new(false);
    let c_trigger = Stateful::new(false);
    let mnt_cnt = Stateful::new(0);
    let c_p_trigger = p_trigger.clone_writer();
    let c_c_trigger = c_trigger.clone_writer();
    let mnt_cnt2 = mnt_cnt.clone_reader();

    let w = fn_widget! {
      pipe!(*$p_trigger).map(move |_| {
        @MockBox {
          size: Size::zero(),
          on_mounted: move |_| *$mnt_cnt.write() +=1,
          @{
            pipe!(*$c_trigger).map(move |_| {
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| *$mnt_cnt.write() +=1,
              }
            })
          }
        }
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*mnt_cnt2.read(), 2);

    {
      // trigger the parent update
      *c_p_trigger.write() = true;
      // then trigger the child update.
      *c_c_trigger.write() = true;
    }
    wnd.draw_frame();
    assert_eq!(*mnt_cnt2.read(), 4);

    // old pipe should be unsubscribed.
    *c_p_trigger.write() = true;
    *c_c_trigger.write() = true;
    wnd.draw_frame();
    assert_eq!(*mnt_cnt2.read(), 6);
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

    fn build(task: Writer<Task>) -> impl WidgetBuilder {
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

    #[derive(Declare, Query)]
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
      let tree = wnd.widget_tree.borrow();
      let root = tree.content_root();
      root.children(&tree.arena).count()
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

    fn tree_arena(wnd: &TestWindow) -> Ref<TreeArena> {
      let tree = wnd.widget_tree.borrow();
      Ref::map(tree, |t| &t.arena)
    }

    let grandson_id = {
      let arena = tree_arena(&wnd);
      let root = wnd.widget_tree.borrow().content_root();
      root
        .first_child(&arena)
        .unwrap()
        .first_child(&arena)
        .unwrap()
    };

    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(&tree_arena(&wnd)));

    c_child.write().take();
    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(&tree_arena(&wnd)));

    *c_child_destroy_until.write() = true;
    wnd.draw_frame();
    assert!(grandson_id.is_dropped(&tree_arena(&wnd)));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn value_pipe() {
    reset_test_env!();
    let hit = State::value(-1);

    let v = BoxPipe::value(0);
    let (v, s) = v.into_pipe().unzip();

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
      let w = pipe!(*$v).map(move |_| Void.build(ctx!()));
      Widget::child_from(w, ctx!())
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
    let pipe_trigger = Stateful::new(0);
    let inner_pipe_trigger = Stateful::new(0);
    let c_pipe_trigger = pipe_trigger.clone_writer();
    let c_inner_pipe_trigger = inner_pipe_trigger.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$pipe_trigger).map(move |w| {
            let pipe_parent = pipe!(*$inner_pipe_trigger)
              .map(move |h| @MockBox { size: Size::new(w as f32, h as f32) });
            @$pipe_parent { @Void {} }
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
}
