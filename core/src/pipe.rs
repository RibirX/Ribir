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
  impl_proxy_render,
  prelude::*,
  ticker::FrameMsg,
  widget::{Query, Render, StrictBuilder, Widget, WidgetBuilder, WidgetId, WidgetTree},
  window::WindowId,
};

/// A value that can be subscribed its continuous change from the observable
/// stream.
pub struct Pipe<V> {
  value: V,
  observable: BoxOp<'static, (ModifyScope, V), Infallible>,
}

macro_rules! new_frame_sampler {
  ($ctx: ident) => {
    $ctx
      .window()
      .frame_tick_stream()
      .filter(|f| matches!(f, FrameMsg::NewFrame(_)))
  };
}

impl<V> Pipe<V> {
  #[inline]
  pub fn new(init: V, observable: BoxOp<'static, (ModifyScope, V), Infallible>) -> Self {
    Self { value: init, observable }
  }

  /// map the inner observable stream to another observable that emit same type
  /// value.
  pub fn stream_map<R>(
    self,
    f: impl FnOnce(BoxOp<'static, (ModifyScope, V), Infallible>) -> R,
  ) -> Pipe<V>
  where
    R: BoxIt<BoxOp<'static, (ModifyScope, V), Infallible>>,
  {
    let Self { value, observable } = self;
    let observable = f(observable).box_it();
    Pipe { value, observable }
  }

  /// Creates a new `Pipe` which calls a closure on each element and
  /// uses its return as the value.
  pub fn map<R: 'static, F>(self, mut f: F) -> Pipe<R>
  where
    F: FnMut(V) -> R + 'static,
    V: 'static,
  {
    let Self { value, observable } = self;
    Pipe {
      value: f(value),
      observable: observable.map(move |(scope, v)| (scope, f(v))).box_it(),
    }
  }

  /// Unzip the `Pipe` into its inner value and the changes stream of the
  /// value.
  #[inline]
  pub fn unzip(self) -> (V, BoxOp<'static, (ModifyScope, V), Infallible>) {
    (self.value, self.observable)
  }

  #[inline]
  pub fn value(&self) -> &V { &self.value }

  #[inline]
  pub fn value_mut(&mut self) -> &mut V { &mut self.value }
}

impl<W: Into<Widget> + 'static> StrictBuilder for Pipe<W> {
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
    let (v, modifies) = self.unzip();
    let id = v.into().build(ctx);
    let id_share = Sc::new(Cell::new(id));

    let mut pipe_node = None;
    id.wrap_node(&mut ctx.tree.borrow_mut().arena, |r| {
      let p = PipeNode::new(r);
      pipe_node = Some(p.clone());
      Box::new(p)
    });

    let id_share2 = id_share.clone();
    let handle = ctx.handle();
    let wnd_id = ctx.window().id();
    let unsub = modifies
      // Collects all the subtrees need to be regenerated before executing the regeneration in the
      // `subscribe` method. Because the `sampler` will delay the `subscribe` until a new frame
      // start.
      .filter(|(scope, _)| scope.contains(ModifyScope::FRAMEWORK))
      .tap(move |_| {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          wnd.mark_widgets_regenerating(id_share2.get(), None)
        }
      })
      .sample(new_frame_sampler!(ctx))
      .subscribe(move |(_, v)| {
        handle.with_ctx(|ctx| {
          let id = id_share.get();

          // async clean the mark when all regenerating is done to avoid other pipe
          // regenerate in the regenerating scope.
          let wnd = ctx.window();
          AppCtx::spawn_local(async move { wnd.remove_regenerating_mark(id) }).unwrap();

          if !ctx.window().is_in_another_regenerating(id) {
            let new_id = v.into().build(ctx);

            // We use a `PipeNode` wrap the initial widget node, so if the widget node is
            // not a `PipeNode` means the node is attached some data, we need to keep the
            // data attached when it replaced by the new widget, because the data is the
            // static stuff.
            //
            // Only pipe widget as a normal widget need to do this, because compose child
            // widget not support pipe object as its child if it's not a normal widget. Only
            // compose child widget has the ability to apply new logic on a widget that
            // built.
            if let Some(pn) = pipe_node.as_mut() {
              let mut tree = ctx.tree.borrow_mut();
              if !id.assert_get(&tree.arena).is::<PipeNode>() {
                let [old_node, new_node] = tree.get_many_mut(&[id, new_id]);

                std::mem::swap(pn.as_mut(), new_node.as_widget_mut());
                std::mem::swap(old_node, new_node);
              } else {
                // we know widget node not attached data, we can not care about it now.
                pipe_node.take();
              }
            }

            update_key_status_single(id, new_id, ctx);

            ctx.insert_after(id, new_id);
            ctx.dispose_subtree(id);
            ctx.on_subtree_mounted(new_id);
            id_share.set(new_id);
            ctx.mark_dirty(new_id)
          }
        });
      });
    attach_unsubscribe_guard(id, ctx.window().id(), unsub);

    id
  }
}

impl<W: Into<Widget> + 'static> WidgetBuilder for Pipe<Option<W>> {
  fn build(self, ctx: &crate::context::BuildCtx) -> WidgetId {
    self
      .map(|w| w.map_or_else(|| Widget::from(Void), |w| w.into()))
      .strict_build(ctx)
  }
}

impl<W: SingleParent + 'static> SingleParent for Pipe<W> {
  fn append_child(self, child: WidgetId, ctx: &BuildCtx) -> WidgetId {
    let (v, modifies) = self.unzip();
    let p = v.append_child(child, ctx);
    let rg = half_to_close_interval(p..child, &ctx.tree.borrow());
    update_pipe_parent(rg, modifies, ctx, |new_p, old_p, ctx| {
      let child = old_p.single_child(&ctx.tree.borrow().arena).unwrap();
      new_p.append_child(child, ctx)
    });
    p
  }
}

impl<W: MultiParent + StrictBuilder + 'static> MultiParent for Pipe<W> {
  fn append_children(self, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId {
    // if children is empty, we can let the pipe parent as the whole subtree.
    if children.is_empty() {
      self.strict_build(ctx)
    } else {
      let (v, modifies) = self.unzip();
      let first_child = children[0];
      let p = v.append_children(children, ctx);
      let rg = half_to_close_interval(p..first_child, &ctx.tree.borrow());
      update_pipe_parent(rg, modifies, ctx, |new_p, old_p, ctx| {
        let children = old_p.children(&ctx.tree.borrow().arena).collect::<Vec<_>>();
        new_p.append_children(children, ctx)
      });
      p
    }
  }
}

impl<W: SingleChild + WidgetBuilder + 'static> SingleParent for Pipe<Option<W>> {
  fn append_child(self, child: WidgetId, ctx: &BuildCtx) -> WidgetId {
    let handle = ctx.handle();
    self
      .map(move |p| {
        handle
          .with_ctx(|ctx| {
            if let Some(p) = p {
              BoxedSingleParent::new(p, ctx)
            } else {
              BoxedSingleParent::new(Void, ctx)
            }
          })
          .expect("Context not available")
      })
      .append_child(child, ctx)
  }
}

fn half_to_close_interval(rg: Range<WidgetId>, tree: &WidgetTree) -> Range<WidgetId> {
  rg.start..rg.end.parent(&tree.arena).unwrap()
}

fn update_pipe_parent<W: 'static>(
  // The range of the pipe parent widget ids.
  parent: Range<WidgetId>,
  // transplant the children of the old parent to the new widget.
  modifies: BoxOp<'static, (ModifyScope, W), Infallible>,
  ctx: &BuildCtx,
  transplant: impl Fn(W, WidgetId, &BuildCtx) -> WidgetId + 'static,
) {
  let id_share = Sc::new(RefCell::new(parent.clone()));
  let id_share2 = id_share.clone();
  let handle = ctx.handle();
  let wnd_id = ctx.window().id();
  let unsub = modifies
    .filter(|(scope, _)| scope.contains(ModifyScope::FRAMEWORK))
    .tap(move |_| {
      if let Some(wnd) = AppCtx::get_window(wnd_id) {
        let rg = id_share2.borrow().clone();
        wnd.mark_widgets_regenerating(rg.start, Some(rg.end))
      }
    })
    .sample(new_frame_sampler!(ctx))
    .subscribe(move |(_, v)| {
      handle.with_ctx(|ctx| {
        let rg = id_share.borrow().clone();

        let wnd = ctx.window();
        // async clean the mark when all regenerating is done to avoid other pipe
        // regenerate in the regenerating scope.
        AppCtx::spawn_local(async move { wnd.remove_regenerating_mark(rg.start) }).unwrap();

        if !ctx.window().is_in_another_regenerating(rg.start) {
          let first_child = rg.end.first_child(&ctx.tree.borrow().arena).unwrap();
          let p = transplant(v, rg.end, ctx);
          let new_rg = half_to_close_interval(p..first_child, &ctx.tree.borrow());

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
    });

  attach_unsubscribe_guard(parent.start, ctx.window().id(), unsub);
}

impl<W: 'static> Pipe<W> {
  pub(crate) fn build_multi(self, vec: &mut Vec<WidgetId>, ctx: &BuildCtx)
  where
    W: IntoIterator,
    W::Item: WidgetBuilder,
  {
    fn build_multi(
      v: impl IntoIterator<Item = impl WidgetBuilder>,
      ctx: &BuildCtx,
    ) -> Vec<WidgetId> {
      let mut ids = v.into_iter().map(|w| w.build(ctx)).collect::<Vec<_>>();

      if ids.is_empty() {
        ids.push(Void.strict_build(ctx));
      }

      ids
    }

    let (m, modifies) = self.unzip();

    let ids = build_multi(m, ctx);
    let first_id = ids[0];
    vec.extend(&ids);

    let ids_share = Sc::new(RefCell::new(ids));
    let id_share2 = ids_share.clone();
    let wnd_id = ctx.window().id();
    let handle = ctx.handle();
    let unsub = modifies
      .filter(|(scope, _)| scope.contains(ModifyScope::FRAMEWORK))
      // Collects all the subtrees need to be regenerated before executing the regeneration in the
      // `subscribe` method. Because the `sampler` will delay the `subscribe` until a new frame
      // start.
      .tap(move |_| {
        if let Some(wnd) = AppCtx::get_window(wnd_id) {
          for id in id_share2.borrow().iter() {
            wnd.mark_widgets_regenerating(*id, None)
          }
        }
      })
      .sample(new_frame_sampler!(ctx))
      .box_it()
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
            let new = build_multi(m, ctx);

            update_key_state_multi(old.iter().copied(), new.iter().copied(), ctx);

            new.iter().for_each(|w| ctx.insert_after(old[0], *w));
            old.iter().for_each(|id| ctx.dispose_subtree(*id));
            new.iter().for_each(|w| {
              ctx.on_subtree_mounted(*w);
              ctx.mark_dirty(*w)
            });
            *old = new;
          }
        });
      });
    attach_unsubscribe_guard(first_id, ctx.window().id(), unsub);
  }
}

fn attach_unsubscribe_guard(id: WidgetId, wnd: WindowId, unsub: impl Subscription + 'static) {
  AppCtx::spawn_local(async move {
    let Some(wnd) = AppCtx::get_window(wnd) else {
      unsub.unsubscribe();
      return;
    };
    let mut tree = wnd.widget_tree.borrow_mut();

    if tree.root() != id {
      let guard = unsub.unsubscribe_when_dropped();
      // auto unsubscribe when the widget is not a root and its parent is None.
      if let Some(p) = id.parent(&tree.arena) {
        p.wrap_node(&mut tree.arena, |d| {
          Box::new(AnonymousWrapper::new(d, Box::new(guard)))
        })
      }
    }
  })
  .unwrap();
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

impl<W: SingleChild> SingleChild for Pipe<W> {}
impl<W: MultiChild> MultiChild for Pipe<W> {}

/// `PipeNode` just use to wrap a `Box<dyn Render>`, and provide a choice to
/// change the inner `Box<dyn Render>` by `UnsafeCell` at a safe time. It's
/// transparent except the `Pipe` widget.
#[derive(Clone)]
struct PipeNode(Sc<UnsafeCell<Box<dyn Render>>>);

impl PipeNode {
  fn new(value: Box<dyn Render>) -> Self { Self(Sc::new(UnsafeCell::new(value))) }

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
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.as_ref().query_inside_first(type_id, callback)
  }

  fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.as_ref().query_outside_first(type_id, callback)
  }
}

impl_proxy_render!(proxy as_ref(), PipeNode);

#[cfg(test)]
mod tests {
  use std::{
    cell::{Cell, Ref},
    rc::Rc,
  };

  use crate::{
    builtin_widgets::key::{AnyKey, KeyChange},
    impl_query_self_only,
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

    // trigger the parent update
    *c_p_trigger.write() = true;
    wnd.run_frame_tasks();
    // then trigger the child update.
    *c_c_trigger.write() = true;

    wnd.draw_frame();
    assert_eq!(*mnt_cnt2.read(), 4);
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
    let w: Widget = fn_widget! {
      @MockMulti {
        @ {
          pipe!($v.clone()).map(move |v| {
            v.into_iter().map(move |(i, c)| {
              let mut key = @KeyWidget { key: i, value: c };
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
    }
    .into();

    // 1. 3 item enter
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    let expect_vec = ['1', '2', '3'];
    assert_eq!((*enter_list.state_ref()).len(), 3);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    (*enter_list.state_ref()).clear();

    // 2. add 1 item
    c_v.write().push((4, '4'));
    wnd.on_wnd_resize_event(ZERO_SIZE);
    wnd.draw_frame();

    let expect_vec = ['4'];
    assert_eq!((*enter_list.state_ref()).len(), 1);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    (*enter_list.state_ref()).clear();

    // 3. update the second item
    c_v.write()[1].1 = 'b';
    wnd.draw_frame();

    let expect_vec = [];
    assert_eq!((*enter_list.state_ref()).len(), 0);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );

    let expect_vec = ['b'];
    assert_eq!((*update_list.state_ref()).len(), 1);
    assert!(
      (*update_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.state_ref(), KeyChange(Some('2'), 'b'));
    (*update_list.state_ref()).clear();

    // 4. remove the second item
    c_v.write().remove(1);
    wnd.draw_frame();
    let expect_vec = vec!['b'];
    assert_eq!((*leave_list.state_ref()), expect_vec);
    assert_eq!((*leave_list.state_ref()).len(), 1);
    assert!(
      (*leave_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    (*leave_list.state_ref()).clear();

    // 5. update the first item
    c_v.write()[0].1 = 'a';
    wnd.draw_frame();

    assert_eq!((*enter_list.state_ref()).len(), 0);

    let expect_vec = ['a'];
    assert_eq!((*update_list.state_ref()).len(), 1);
    assert!(
      (*update_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.state_ref(), KeyChange(Some('1'), 'a'));
    (*update_list.state_ref()).clear();
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

    fn build(task: Writer<Task>) -> Widget {
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
      .into()
    }

    #[derive(Declare2)]
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

    impl_query_self_only!(TaskWidget);

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
    tasks.state_ref()[0].state_ref().pin = true;
    removed.push(tasks.state_ref().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 2);
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 2);

    // the remove pined widget will paint and no layout when no changed
    let first_layout_cnt = removed[0].state_ref().layout_cnt.get();
    tasks.state_ref().get(0).unwrap().state_ref().pin = true;
    removed.push(tasks.state_ref().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 1);
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 3);
    assert_eq!(removed[1].state_ref().paint_cnt.get(), 3);
    assert_eq!(removed[0].state_ref().layout_cnt.get(), first_layout_cnt);

    // the remove pined widget only mark self dirty
    let first_layout_cnt = removed[0].state_ref().layout_cnt.get();
    let second_layout_cnt = removed[1].state_ref().layout_cnt.get();
    let host_layout_cnt = tasks.state_ref()[0].state_ref().layout_cnt.get();
    removed[0].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(
      removed[0].state_ref().layout_cnt.get(),
      first_layout_cnt + 1
    );
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 4);
    assert_eq!(removed[1].state_ref().layout_cnt.get(), second_layout_cnt);
    assert_eq!(
      tasks.state_ref()[0].state_ref().layout_cnt.get(),
      host_layout_cnt
    );

    // when unpined, it will no paint anymore
    removed[0].state_ref().pin = false;
    wnd.draw_frame();
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 4);
    assert_eq!(removed[1].state_ref().paint_cnt.get(), 5);

    // after removed, it will no paint and layout anymore
    let first_layout_cnt = removed[0].state_ref().layout_cnt.get();
    removed[0].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 4);
    assert_eq!(removed[1].state_ref().paint_cnt.get(), 5);
    assert_eq!(removed[0].state_ref().layout_cnt.get(), first_layout_cnt);

    // other pined widget is work fine.
    let first_layout_cnt = removed[0].state_ref().layout_cnt.get();
    let second_layout_cnt = removed[1].state_ref().layout_cnt.get();
    removed[1].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].state_ref().paint_cnt.get(), 4);
    assert_eq!(removed[1].state_ref().paint_cnt.get(), 6);
    assert_eq!(removed[0].state_ref().layout_cnt.get(), first_layout_cnt);
    assert_eq!(
      removed[1].state_ref().layout_cnt.get(),
      second_layout_cnt + 1,
    );
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
