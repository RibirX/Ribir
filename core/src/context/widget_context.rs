use painter::{Point, Rect};

use crate::prelude::{widget_tree::WidgetTree, Context, LayoutStore, QueryOrder, WidgetId};

/// common action for all context of widget.
pub trait WidgetCtx {
  /// Return parent of widget of this context.
  fn parent(&self) -> Option<WidgetId>;
  /// Return parent of widget `w`.
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId>;
  /// Return the single child of `widget`.
  /// # Panic
  /// panic if widget have multi child.
  fn single_child(&self) -> Option<WidgetId>;
  /// Return the box rect of the single child of widget.
  /// # Panic
  /// panic if widget have multi child.
  fn single_child_box(&self) -> Option<Rect> {
    self.single_child().and_then(|c| self.widget_box_rect(c))
  }
  /// Return the widget box rect of the widget of the context.
  fn box_rect(&self) -> Option<Rect>;
  /// Return the box rect of the widget `wid` point to.
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect>;
  /// Translates the global window coordinate pos to widget coordinates.
  fn map_to_global(&self, pos: Point) -> Point;
  /// Translates the global screen coordinate pos to widget coordinates.
  fn map_from_global(&self, pos: Point) -> Point;
  /// Translates the widget pos to the coordinate system of `parent`.
  fn map_to_parent(&self, pos: Point) -> Point;
  /// Translates the widget pos from the coordinate system of parent to this
  /// widget system.
  fn map_from_parent(&self, pos: Point) -> Point;
  /// Translates the widget pos to the coordinate system of `w`.
  fn map_to(&self, pos: Point, w: WidgetId) -> Point;
  /// Translates the widget pos from the coordinate system of `w` to this widget
  /// system.
  fn map_from(&self, pos: Point, w: WidgetId) -> Point;
  /// Returns some reference to the inner value if the widget back of `id` is
  /// type `T`, or `None` if it isn't.
  fn query_widget_type<T: 'static>(&self, id: WidgetId, callback: impl FnOnce(&T));
}

pub(crate) fn map_to_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  // todo: should effect by transform widget.
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos + rect.min().to_vector())
}

pub(crate) fn map_from_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos - rect.min().to_vector())
  // todo: should effect by transform widget.
}

pub(crate) fn map_to_global(id: WidgetId, pos: Point, tree: &WidgetTree) -> Point {
  let ctx = tree.context();
  let ctx = ctx.borrow();
  id.ancestors(tree)
    .fold(pos, |pos, p| map_to_parent(p, pos, &ctx.layout_store))
}

pub(crate) fn map_from_global(id: WidgetId, pos: Point, tree: &WidgetTree) -> Point {
  let stack = id.ancestors(tree).collect::<Vec<_>>();
  let binding = tree.context();
  let ctx = binding.borrow();
  stack
    .iter()
    .rev()
    .fold(pos, |pos, p| map_from_parent(*p, pos, &ctx.layout_store))
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;

  fn widget_tree(&self) -> &WidgetTree;

  /// The return `Context` should ba same one in `WidgetTree` return by
  /// [`widget_tree`](WidgetCtxImpl::widget_tree), this method help
  /// implementation can cache the context to avoid frequently upgrade weak
  /// pointer from `WidgetTree`, see the implementation of
  /// [`WidgetTree::Context`]!.
  fn context(&self) -> Option<&Context>;
}

impl<T: WidgetCtxImpl> WidgetCtx for T {
  #[inline]
  fn parent(&self) -> Option<WidgetId> { self.id().parent(self.widget_tree()) }

  #[inline]
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId> { w.parent(self.widget_tree()) }

  #[inline]
  fn single_child(&self) -> Option<WidgetId> { self.id().single_child(self.widget_tree()) }

  #[inline]
  fn box_rect(&self) -> Option<Rect> { self.widget_box_rect(self.id()) }

  #[inline]
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    inspect_on_context(self, |ctx| ctx.layout_store.layout_box_rect(wid))
  }

  #[inline]
  fn map_to_global(&self, pos: Point) -> Point { map_to_global(self.id(), pos, self.widget_tree()) }

  #[inline]
  fn map_from_global(&self, pos: Point) -> Point {
    map_from_global(self.id(), pos, self.widget_tree())
  }

  #[inline]
  fn map_to_parent(&self, pos: Point) -> Point {
    inspect_on_context(self, |ctx| map_to_parent(self.id(), pos, &ctx.layout_store))
  }

  #[inline]
  fn map_from_parent(&self, pos: Point) -> Point {
    inspect_on_context(self, |ctx| {
      map_from_parent(self.id(), pos, &ctx.layout_store)
    })
  }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    map_from_global(w, global, self.widget_tree())
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = map_to_global(w, pos, self.widget_tree());
    self.map_from_global(global)
  }

  #[inline]
  fn query_widget_type<W: 'static>(&self, id: WidgetId, callback: impl FnOnce(&W)) {
    id.assert_get(self.widget_tree())
      .query_on_first_type(QueryOrder::OutsideFirst, callback);
  }
}

fn inspect_on_context<T: WidgetCtxImpl, F: FnOnce(&Context) -> R, R>(w_ctx: &T, f: F) -> R {
  if let Some(ctx) = w_ctx.context() {
    f(ctx)
  } else {
    let binding = w_ctx.widget_tree().context();
    let ctx = binding.borrow();
    f(&*ctx)
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;

  #[test]
  fn map_self_eq_self() {
    impl<'a> WidgetCtxImpl for (WidgetId, &'a Context) {
      fn id(&self) -> WidgetId { self.0 }

      fn widget_tree(&self) -> &WidgetTree { &self.1.widget_tree }

      fn layout_store(&self) -> &LayoutStore { &self.1.layout_store }
    }

    let w = widget! {
      SizedBox {
        size: Size::zero(),
        margin: EdgeInsets::all(2.),
      }
    };
    let mut wnd = Window::without_render(w.into_widget(), Size::zero());
    wnd.render_ready();

    let ctx = wnd.context();
    let root = ctx.widget_tree.root();
    let pos = Point::zero();
    let child = root.single_child(&ctx.widget_tree).unwrap();

    let w_ctx = (child, ctx);
    assert_eq!(w_ctx.map_from(pos, child), pos);
    assert_eq!(w_ctx.map_to(pos, child), pos);
  }
}
