use rxrust::{
  ops::box_it::BoxOp,
  prelude::{BoxIt, ObservableExt, ObservableItem},
  subscription::Subscription,
};
use std::{
  cell::{Cell, RefCell},
  convert::Infallible,
  rc::Rc,
};

use crate::{
  builtin_widgets::{key::AnyKey, Void},
  context::BuildCtx,
  data_widget::attach_to_id,
  prelude::{AnonymousData, DataWidget, Multi, MultiChild, SingleChild},
  widget::{QueryOrder, Render, Widget, WidgetBuilder, WidgetId},
};

/// A value that can be subscribed its continuous change from the observable
/// stream.
pub struct Pipe<V> {
  value: V,
  observable: BoxOp<'static, V, Infallible>,
}

impl<V> Pipe<V> {
  #[inline]
  pub fn new(init: V, observable: BoxOp<'static, V, Infallible>) -> Self {
    Self { value: init, observable }
  }

  /// map the inner observable stream to another observable that emit same type
  /// value.
  pub fn stream_map<R>(self, f: impl FnOnce(BoxOp<'static, V, Infallible>) -> R) -> Pipe<V>
  where
    R: BoxIt<BoxOp<'static, V, Infallible>>,
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
      observable: observable.map(f).box_it(),
    }
  }

  /// Unzip the `Pipe` into its inner value and the changes stream of the
  /// value.
  #[inline]
  pub fn unzip(self) -> (V, BoxOp<'static, V, Infallible>) { (self.value, self.observable) }

  #[inline]
  pub fn value(&self) -> &V { &self.value }

  #[inline]
  pub fn value_mut(&mut self) -> &mut V { &mut self.value }
}

impl<W: Into<Widget>> WidgetBuilder for Pipe<W> {
  #[inline]
  fn build(self, ctx: &crate::context::BuildCtx) -> WidgetId {
    let (v, modifies) = self.unzip();
    let id = v.into().build(ctx);
    let id_share = Rc::new(Cell::new(id));
    let handle = ctx.handle();
    let h = modifies
      .subscribe(move |v| {
        handle.with_ctx(|ctx| {
          let id = id_share.get();
          let ctx = ctx.force_as_mut();
          let new_id = v.into().build(ctx);

          update_key_status_single(id, new_id, ctx);

          ctx.insert_after(id, new_id);
          ctx.dispose_subtree(id);
          ctx.on_subtree_mounted(new_id);
          id_share.set(new_id);
          ctx.mark_dirty(new_id)
        });
      })
      .unsubscribe_when_dropped();

    let h = AnonymousData::new(Box::new(h));
    attach_to_id(id, ctx.force_as_mut(), |d| Box::new(DataWidget::new(d, h)));

    id
  }
}

impl<R: Into<Box<dyn Render>>> Pipe<R> {
  pub(crate) fn build_as_render_parent(self, ctx: &mut BuildCtx) -> WidgetId {
    let (v, modifies) = self.unzip();
    let id = ctx.alloc_widget(v.into());
    let id_share = Rc::new(Cell::new(id));
    let handle = ctx.handle();
    let h = modifies
      .subscribe(move |v| {
        handle.with_ctx(|ctx| {
          let id = id_share.get();
          let ctx = ctx.force_as_mut();
          let new_id = ctx.alloc_widget(v.into());

          update_key_status_single(id, new_id, ctx);
          let mut cursor = id.first_child(&ctx.tree.arena);
          while let Some(c) = cursor {
            cursor = c.next_sibling(&ctx.tree.arena);
            ctx.append_child(new_id, c);
          }

          ctx.insert_after(id, new_id);
          ctx.dispose_subtree(id);

          ctx.on_widget_mounted(new_id);
          id_share.set(new_id);
          ctx.mark_dirty(new_id);
        });
      })
      .unsubscribe_when_dropped();

    let h = AnonymousData::new(Box::new(h));
    attach_to_id(id, ctx.force_as_mut(), |d| Box::new(DataWidget::new(d, h)));

    id
  }
}

impl<W> Pipe<Multi<W>> {
  pub(crate) fn build_multi(self, vec: &mut Vec<WidgetId>, ctx: &mut BuildCtx)
  where
    W: IntoIterator,
    W::Item: Into<Widget>,
  {
    fn build_multi(
      v: Multi<impl IntoIterator<Item = impl Into<Widget>>>,
      ctx: &mut BuildCtx,
      s_guard: impl Clone + 'static,
    ) -> Box<[WidgetId]> {
      let mut ids = v
        .into_inner()
        .into_iter()
        .map(|w| w.into().build(ctx))
        .collect::<Vec<_>>();

      if ids.is_empty() {
        ids.push(Void.build(ctx));
      }
      for id in &ids {
        attach_to_id(*id, ctx, |d| {
          let h = AnonymousData::new(Box::new(s_guard.clone()));
          Box::new(DataWidget::new(d, h))
        });
      }

      ids.into_boxed_slice()
    }

    let s_guard = Rc::new(RefCell::new(None));
    let (m, modifies) = self.unzip();

    let ids = build_multi(m, ctx, s_guard.clone());
    vec.extend(&*ids);

    let ids_share = Rc::new(RefCell::new(ids));

    let handle = ctx.handle();

    let s_guard2 = s_guard.clone();
    let guard = modifies
      .subscribe(move |m| {
        handle.with_ctx(|ctx| {
          let ctx = ctx.force_as_mut();
          let old = ids_share.borrow();
          let new = build_multi(m, ctx, s_guard.clone());

          update_key_state_multi(&new, &old, ctx);

          new.iter().for_each(|w| ctx.insert_after(old[0], *w));
          old.iter().for_each(|id| ctx.dispose_subtree(*id));
          new.iter().for_each(|w| {
            ctx.on_subtree_mounted(*w);
            ctx.mark_dirty(*w)
          });
        });
      })
      .unsubscribe_when_dropped();

    s_guard2.borrow_mut().replace(guard);
  }
}

fn update_key_status_single(new: WidgetId, old: WidgetId, ctx: &BuildCtx) {
  inspect_key(old, ctx, |old_key| {
    inspect_key(new, ctx, |new_key| {
      if old_key.key() == new_key.key() {
        new_key.record_before_value(old_key);
      } else {
        old_key.disposed();
        new_key.mounted();
      }
    })
  })
}

fn update_key_state_multi(old: &[WidgetId], new: &[WidgetId], ctx: &BuildCtx) {
  let mut old_key_list = ahash::HashMap::default();

  for o in old {
    inspect_key(*o, ctx, |old_key: &dyn AnyKey| {
      let key = old_key.key();
      old_key_list.insert(key, *o);
    });
  }

  for n in new {
    inspect_key(*n, ctx, |new_key: &dyn AnyKey| {
      let key = &new_key.key();
      if let Some(o) = old_key_list.get(key) {
        inspect_key(*o, ctx, |old_key_widget: &dyn AnyKey| {
          new_key.record_before_value(old_key_widget)
        });
        old_key_list.remove(key);
      } else {
        new_key.mounted();
      }
    });
  }

  old_key_list
    .values()
    .for_each(|o| inspect_key(*o, ctx, |old_key| old_key.disposed()));
}

fn inspect_key(id: WidgetId, ctx: &BuildCtx, mut cb: impl FnMut(&dyn AnyKey)) {
  #[allow(clippy::borrowed_box)]
  ctx
    .assert_get(id)
    .query_on_first_type::<Box<dyn AnyKey>, _>(QueryOrder::OutsideFirst, |key_widget| {
      cb(&**key_widget)
    });
}

impl<W: SingleChild> SingleChild for Pipe<W> {}
impl<W: MultiChild> MultiChild for Pipe<W> {}
