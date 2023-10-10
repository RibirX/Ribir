use ribir_algo::Sc;
use rxrust::{
  ops::box_it::BoxOp,
  prelude::{BoxIt, ObservableExt, ObservableItem},
  subscription::Subscription,
};
use std::{
  cell::{Cell, RefCell, UnsafeCell},
  convert::Infallible,
  ops::{Deref, Range},
};

use crate::{
  builtin_widgets::{key::AnyKey, Void},
  context::{AppCtx, BuildCtx},
  prelude::*,
  render_helper::{RenderProxy, RenderTarget},
  ticker::FrameMsg,
  widget::{Render, RenderBuilder, Widget, WidgetId},
};

type ValueStream<V> = BoxOp<'static, (ModifyScope, V), Infallible>;

/// A trait for a value that can be subscribed its continuous modifies.
pub trait Pipe {
  type Value;
  /// Unzip the `Pipe` into its inner value and the changes stream of the
  /// value.
  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>);

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>);

  /// Maps an `Pipe<Value=T>` to `Pipe<Value=T>` by applying a function to the
  /// continuous value
  fn map<R>(self, f: impl FnMut(Self::Value) -> R + 'static) -> MapPipe<R, Self>
  where
    Self: Sized,
  {
    MapPipe { source: self, f: Box::new(f) }
  }
}

pub(crate) trait InnerPipe: Pipe {
  /// unzip the pipe value stream with a tick sample, only for build widget use.
  fn widget_unzip(
    self,
    tap_before_all: impl Fn(&Window) + 'static,
    ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>);

  /// Chain more operations on the pipe value stream by applying the `f` on the
  /// final value stream. This a lazy operation, it will not execute the `f`
  /// until the pipe is be subscribed.
  fn final_stream_chain<F>(
    self,
    f: impl FnOnce(ValueStream<Self::Value>) -> ValueStream<Self::Value> + 'static,
  ) -> FinalChain<Self::Value, Self>
  where
    Self: Sized,
  {
    FinalChain { source: self, f: Box::new(f) }
  }

  fn build(
    self,
    ctx: &BuildCtx,
    build: impl Fn(Self::Value, &BuildCtx) -> Widget + 'static,
  ) -> Widget
  where
    Self: Sized,
    Self::Value: 'static,
  {
    let id_share = Sc::new(Cell::new(ctx.tree.borrow().root()));
    let id_share2 = id_share.clone();
    let (w, modifies) = self.widget_unzip(
      move |wnd| wnd.mark_widgets_regenerating(id_share2.get(), None),
      ctx,
    );
    let w = build(w, ctx);
    id_share.set(w.id());

    let mut pipe_node = PipeNode::share_capture(w.id(), ctx);
    let handle = ctx.handle();

    let u = modifies
      .subscribe(move |(_, w)| {
        handle.with_ctx(|ctx| {
          let id = id_share.get();

          // async clean the mark when all regenerating is done to avoid other pipe
          // regenerate in the regenerating scope.
          let wnd = ctx.window();
          AppCtx::spawn_local(async move { wnd.remove_regenerating_mark(id) }).unwrap();

          if !ctx.window().is_in_another_regenerating(id) {
            let new_id = build(w, ctx).consume();
            pipe_node.transplant_to(id, new_id, ctx);
            update_key_status_single(id, new_id, ctx);

            ctx.insert_after(id, new_id);
            ctx.dispose_subtree(id);
            ctx.on_subtree_mounted(new_id);
            id_share.set(new_id);
            ctx.mark_dirty(new_id)
          }
        });
      })
      .unsubscribe_when_dropped();

    w.attach_anonymous_data(u, ctx)
  }

  fn build_multi(
    self,
    vec: &mut Vec<Widget>,
    f: impl Fn(<Self::Value as IntoIterator>::Item, &BuildCtx) -> Widget + 'static,
    ctx: &BuildCtx,
  ) where
    Self::Value: IntoIterator,
    Self: Sized,
  {
    let build_multi = move |widgets: Self::Value, ctx: &BuildCtx| {
      let mut widgets = widgets.into_iter().map(|w| f(w, ctx)).collect::<Vec<_>>();
      if widgets.is_empty() {
        widgets.push(Void.widget_build(ctx));
      }
      widgets
    };

    let ids_share = Sc::new(RefCell::new(vec![]));
    let id_share2 = ids_share.clone();
    let (m, modifies) = self.widget_unzip(
      move |wnd| {
        for id in id_share2.borrow().iter() {
          wnd.mark_widgets_regenerating(*id, None)
        }
      },
      ctx,
    );

    let widgets = build_multi(m, ctx);
    let first = widgets[0].id();
    *ids_share.borrow_mut() = widgets.iter().map(|w| w.id()).collect();
    vec.extend(widgets);

    let mut pipe_node = PipeNode::share_capture(first, ctx);

    let handle = ctx.handle();
    let u = modifies
      .subscribe(move |(_, m)| {
        handle.with_ctx(|ctx| {
          let mut old = ids_share.borrow_mut();
          let removed_subtree = old.clone();

          // async clean the mark when all regenerating is done to avoid other pipe
          // regenerate in the regenerating scope.
          let wnd = ctx.window();
          AppCtx::spawn_local(async move {
            for id in removed_subtree {
              wnd.remove_regenerating_mark(id);
            }
          })
          .unwrap();

          if !ctx.window().is_in_another_regenerating(old[0]) {
            let new = build_multi(m, ctx)
              .into_iter()
              .map(Widget::consume)
              .collect::<Vec<_>>();

            pipe_node.transplant_to(old[0], new[0], ctx);

            update_key_state_multi(old.iter().copied(), new.iter().copied(), ctx);

            new.iter().rev().for_each(|w| ctx.insert_after(old[0], *w));
            old.iter().for_each(|id| ctx.dispose_subtree(*id));
            new.iter().for_each(|w| {
              ctx.on_subtree_mounted(*w);
              ctx.mark_dirty(*w)
            });
            *old = new;
          }
        });
      })
      .unsubscribe_when_dropped();

    first.attach_anonymous_data(u, &mut ctx.tree.borrow_mut().arena);
  }

  fn only_parent_build(
    self,
    ctx: &BuildCtx,
    compose_child: impl FnOnce(Self::Value) -> (Widget, WidgetId),
    transplant: impl Fn(Self::Value, WidgetId, &BuildCtx) -> WidgetId + 'static,
  ) -> Widget
  where
    Self: Sized,
  {
    let root = ctx.tree.borrow().root();
    let id_share = Sc::new(RefCell::new(root..root));
    let id_share2 = id_share.clone();

    let (v, modifies) = self.widget_unzip(
      move |wnd| {
        let rg = id_share2.borrow().clone();
        wnd.mark_widgets_regenerating(rg.start, Some(rg.end))
      },
      ctx,
    );
    let (p, child) = compose_child(v);
    let parent = half_to_close_interval(p.id()..child, ctx);
    let mut pipe_node = PipeNode::share_capture(parent.start, ctx);
    *id_share.borrow_mut() = parent;

    let handle = ctx.handle();

    let u = modifies
      .subscribe(move |(_, w)| {
        handle.with_ctx(|ctx| {
          let rg = id_share.borrow().clone();

          let wnd = ctx.window();
          // async clean the mark when all regenerating is done to avoid other pipe
          // regenerate in the regenerating scope.
          AppCtx::spawn_local(async move { wnd.remove_regenerating_mark(rg.start) }).unwrap();

          if !ctx.window().is_in_another_regenerating(rg.start) {
            let first_child = rg.end.first_child(&ctx.tree.borrow().arena).unwrap();
            let p = transplant(w, rg.end, ctx);
            let new_rg = half_to_close_interval(p..first_child, ctx);
            pipe_node.transplant_to(rg.start, new_rg.start, ctx);

            update_key_status_single(rg.start, new_rg.start, ctx);

            ctx.insert_after(rg.start, new_rg.start);
            ctx.dispose_subtree(rg.start);
            new_rg
              .end
              .ancestors(&ctx.tree.borrow().arena)
              .take_while(|w| w != &new_rg.start)
              .for_each(|p| ctx.on_widget_mounted(p));

            ctx.mark_dirty(new_rg.start);
            *id_share.borrow_mut() = new_rg;
          }
        });
      })
      .unsubscribe_when_dropped();

    p.id()
      .attach_anonymous_data(u, &mut ctx.tree.borrow_mut().arena);
    p
  }
}

pub struct MapPipe<V, S: Pipe> {
  source: S,
  f: Box<dyn FnMut(S::Value) -> V>,
}

pub struct FinalChain<V, S: Pipe<Value = V>> {
  source: S,
  f: Box<dyn FnOnce(ValueStream<V>) -> ValueStream<V>>,
}

impl<S: Pipe, V> MapPipe<V, S> {
  pub fn new(source: S, f: impl FnMut(S::Value) -> V + 'static) -> Self {
    Self { source, f: Box::new(f) }
  }
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
    (
      ModifyScope::empty(),
      ObservableExt::map(self.0, |s| (s, s)).box_it(),
    )
  }

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }
}

impl InnerPipe for ModifiesPipe {
  fn widget_unzip(
    self,
    tap_before_all: impl Fn(&Window) + 'static,
    ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let wnd_id = ctx.window().id();
    let stream = self
      .0
      .filter(|s| s.contains(ModifyScope::FRAMEWORK))
      .tap(move |_| {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          tap_before_all(&wnd)
        }
      })
      .sample(
        ctx
          .window()
          .frame_tick_stream()
          .filter(|f| matches!(f, FrameMsg::NewFrame(_))),
      )
      .map(|s| (s, s))
      .box_it();
    (ModifyScope::empty(), stream)
  }
}

impl<V: 'static, S: Pipe> Pipe for MapPipe<V, S>
where
  S::Value: 'static,
{
  type Value = V;

  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f } = self;
    let (v, stream) = source.unzip();
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }
}

impl<V: 'static, S: InnerPipe> InnerPipe for MapPipe<V, S>
where
  S::Value: 'static,
{
  fn widget_unzip(
    self,
    tap_before_all: impl Fn(&Window) + 'static,
    ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, mut f } = self;
    let (v, stream) = source.widget_unzip(tap_before_all, ctx);
    (f(v), stream.map(move |(s, v)| (s, f(v))).box_it())
  }
}

impl<V, S: Pipe<Value = V>> Pipe for FinalChain<V, S> {
  type Value = V;

  fn unzip(self) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, f } = self;
    let (v, stream) = source.unzip();
    (v, f(stream))
  }

  fn box_unzip(self: Box<Self>) -> (Self::Value, ValueStream<Self::Value>) { (*self).unzip() }
}

impl<V, S: InnerPipe<Value = V>> InnerPipe for FinalChain<V, S> {
  fn widget_unzip(
    self,
    tap_before_all: impl Fn(&Window) + 'static,
    ctx: &BuildCtx,
  ) -> (Self::Value, ValueStream<Self::Value>) {
    let Self { source, f } = self;
    let (v, stream) = source.widget_unzip(tap_before_all, ctx);
    (v, f(stream))
  }
}

crate::widget::multi_build_replace_impl! {
  impl<V: {#} + 'static, S: InnerPipe> {#} for MapPipe<V, S>
  where
    S::Value: 'static,
  {
    fn widget_build(self, ctx: &BuildCtx) -> Widget {
      self.build(ctx, |w, ctx| w.widget_build(ctx))
    }
  }

  impl<V: {#} + 'static, S: InnerPipe<Value = V>> {#} for FinalChain<V, S>
  where
    S::Value: 'static,
  {
    fn widget_build(self, ctx: &BuildCtx) -> Widget {
      self.build(ctx, |w, ctx| w.widget_build(ctx))
    }
  }
}

impl<S> WidgetBuilder for MapPipe<Widget, S>
where
  S: InnerPipe,
  S::Value: 'static,
{
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self.build(ctx, |v, _| v) }
}

impl<S: InnerPipe<Value = Widget>> WidgetBuilder for FinalChain<Widget, S> {
  fn widget_build(self, ctx: &BuildCtx) -> Widget { self.build(ctx, |v, _| v) }
}

macro_rules! pipe_option_to_widget {
  ($name: ident, $ctx: ident) => {
    $name
      .map(|w| {
        move |ctx: &BuildCtx| {
          if let Some(w) = w {
            w.widget_build(ctx)
          } else {
            Void.widget_build(ctx)
          }
        }
      })
      .widget_build($ctx)
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
          let child = old_p.single_child(&ctx.tree.borrow().arena).unwrap();
          let child = Widget::from_id(child, ctx);
          new_p.compose_child(child, ctx).consume()
        },
      )
    }
  };
}

impl<V, S> SingleParent for MapPipe<V, S>
where
  S: InnerPipe,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
{
  single_parent_impl!();
}

impl<V, S> SingleParent for FinalChain<V, S>
where
  S: InnerPipe<Value = V>,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
{
  single_parent_impl!();
}

macro_rules! multi_parent_impl {
  () => {
    fn compose_children(
      self,
      mut children: impl Iterator<Item = Widget>,
      ctx: &BuildCtx,
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
            let children = old_p.children(arena).map(|id| Widget::from_id(id, ctx));
            new_p.compose_children(children.into_iter(), ctx).consume()
          },
        )
      } else {
        self.widget_build(ctx)
      }
    }
  };
}

impl<V, S> MultiParent for MapPipe<V, S>
where
  S: InnerPipe,
  V: MultiParent + RenderBuilder + 'static,
  S::Value: 'static,
{
  multi_parent_impl!();
}

impl<V: MultiParent + RenderBuilder + 'static, S: InnerPipe<Value = V>> MultiParent
  for FinalChain<V, S>
{
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
              BoxedSingleChild::from_id(p.widget_build(ctx))
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

impl<V, S> SingleParent for MapPipe<Option<V>, S>
where
  S: InnerPipe,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
{
  option_single_parent_impl!();
}

impl<V, S> SingleParent for FinalChain<Option<V>, S>
where
  S: InnerPipe<Value = Option<V>>,
  V: SingleParent + RenderBuilder + 'static,
  S::Value: 'static,
{
  option_single_parent_impl!();
}

fn half_to_close_interval(rg: Range<WidgetId>, ctx: &BuildCtx) -> Range<WidgetId> {
  rg.start..rg.end.parent(&ctx.tree.borrow().arena).unwrap()
}

fn update_children_key_status(old: WidgetId, new: WidgetId, ctx: &BuildCtx) {
  let tree = &ctx.tree.borrow().arena;

  match (
    old.first_child(tree),
    old.last_child(tree),
    new.first_child(tree),
    new.last_child(tree),
  ) {
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
  old: impl Iterator<Item = WidgetId>,
  new: impl Iterator<Item = WidgetId>,
  ctx: &BuildCtx,
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
          inspect_key(o, ctx, |old_key| {
            update_key_states(old_key, o, new_key, n, ctx)
          });
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
  old_key: &dyn AnyKey,
  old: WidgetId,
  new_key: &dyn AnyKey,
  new: WidgetId,
  ctx: &BuildCtx,
) {
  new_key.record_prev_key_widget(old_key);
  old_key.record_next_key_widget(new_key);
  update_children_key_status(old, new, ctx)
}

impl<S: Pipe, V: SingleChild> SingleChild for MapPipe<V, S> {}
impl<S: Pipe<Value = V>, V: MultiChild> MultiChild for FinalChain<V, S> {}
impl<S: Pipe, V: SingleChild> SingleChild for MapPipe<Option<V>, S> {}
impl<S: Pipe<Value = Option<V>>, V: MultiChild> MultiChild for FinalChain<Option<V>, S> {}

/// `PipeNode` just use to wrap a `Box<dyn Render>`, and provide a choice to
/// change the inner `Box<dyn Render>` by `UnsafeCell` at a safe time. It's
/// transparent except the `Pipe` widget.
///
/// We use a `PipeNode` wrap the widget node of the pipe, so we can only
/// replace the dynamic part come from the pipe, and keep the static data
/// attached to this node. For example, we attached the unsubscribe handle of
/// the pipe to the first node, and user can attach `key` or `listener` to the
/// widget after `Pipe::build_widget` call.
#[derive(Clone)]
struct PipeNode(Sc<UnsafeCell<Box<dyn Render>>>);

impl PipeNode {
  fn share_capture(id: WidgetId, ctx: &BuildCtx) -> Self {
    let mut pipe_node = None;
    id.wrap_node(&mut ctx.tree.borrow_mut().arena, |r| {
      let p = Self(Sc::new(UnsafeCell::new(r)));
      pipe_node = Some(p.clone());
      Box::new(RenderProxy::new(p))
    });
    // we init before.
    unsafe { pipe_node.unwrap_unchecked() }
  }

  fn transplant_to(&mut self, from: WidgetId, to: WidgetId, ctx: &BuildCtx) {
    let mut tree = ctx.tree.borrow_mut();
    let [old_node, new_node] = tree.get_many_mut(&[from, to]);

    std::mem::swap(self.as_mut(), new_node);
    std::mem::swap(old_node, new_node);
  }

  fn as_ref(&self) -> &dyn Render {
    // safety: see the `PipeNode` document.
    unsafe { &**self.0.get() }
  }

  fn as_mut(&mut self) -> &mut Box<dyn Render> {
    // safety: see the `PipeNode` document.
    unsafe { &mut *self.0.get() }
  }
}

impl Query for PipeNode {
  crate::widget::impl_proxy_query!(as_ref());
}

impl RenderTarget for PipeNode {
  type Target = dyn Render;
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(self.as_ref()) }
}

#[cfg(test)]
mod tests {
  use std::{
    cell::{Cell, Ref},
    rc::Rc,
  };

  use crate::{
    builtin_widgets::key::{AnyKey, KeyChange},
    prelude::*,
    reset_test_env,
    test_helper::*,
    widget::TreeArena,
  };

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
    let ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 2);

    assert_eq!(ids[1], new_ids[1]);
  }

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
    let ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 3);

    assert_eq!(ids[0], new_ids[0]);
    assert_eq!(ids[2], new_ids[2]);
  }

  #[test]
  fn attach_data_to_pipe_widget() {
    reset_test_env!();
    let trigger = Stateful::new(false);
    let c_trigger = trigger.clone_reader();
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
        .root()
        .assert_get(&tree.arena)
        .contain_type::<Box<dyn AnyKey>>()
    );
  }

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
    wnd.on_wnd_resize_event(Size::zero());
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
    wnd.on_wnd_resize_event(ZERO_SIZE);
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
          delay_drop_until: pipe!(!$task.pin),
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

    #[derive(Declare2, Query)]
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
      let root = tree.root();
      root.children(&tree.arena).count()
    }

    let tasks = (0..3)
      .map(|_| Stateful::new(Task::default()))
      .collect::<Vec<_>>();
    let tasks = Stateful::new(tasks);
    let c_tasks = tasks.clone_reader();
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
    tasks.read().get(0).unwrap().write().pin = true;
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
            delay_drop_until: pipe!(*$child_destroy_until),
            @ { pipe!(*$grandson).map(move |_| {
              @MockBox {
                delay_drop_until: pipe!(*$grandson_destroy_until),
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
      let root = wnd.widget_tree.borrow().root();
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
}
