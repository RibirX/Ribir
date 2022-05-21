use painter::{Point, Rect};

use super::Context;
use crate::prelude::{widget_tree::WidgetTree, LayoutStore, QueryOrder, WidgetId};

/// common action for all context of widget.
pub trait WidgetCtx {
  /// Return the single child of `widget`, panic if have more than once child.
  fn single_child(&self) -> Option<WidgetId>;

  /// Return the widget box rect of the widget of the context.
  fn box_rect(&self) -> Option<Rect>;

  /// Return the box rect of the widget `wid` point to.
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect>;

  /// Translates the global window coordinate pos to widget coordinates.
  fn map_to_global(&self, pos: Point) -> Point;

  /// Translates the global screen coordinate pos to widget coordinates.
  fn map_from_global(&self, pos: Point) -> Point;

  /// Translates the render object coordinate pos to the coordinate system of
  /// `parent`.
  fn map_to_parent(&self, pos: Point) -> Point;

  /// Translates the render object coordinate pos from the coordinate system of
  /// parent to this render object coordinate system.
  fn map_from_parent(&self, pos: Point) -> Point;

  /// Translates the render object coordinate pos to the coordinate system of
  /// `w`.
  fn map_to(&self, pos: Point, w: WidgetId) -> Point;

  /// Translates the render object coordinate pos from the coordinate system of
  /// `w` to this render object coordinate system.
  fn map_from(&self, pos: Point, w: WidgetId) -> Point;

  /// Returns some reference to the inner value if the widget back of `id` is
  /// type `T`, or `None` if it isn't.
  fn query_type<T: 'static>(&self, id: WidgetId) -> Option<&T>;
}

pub fn map_to_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  // todo: should effect by transform widget.
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos + rect.min().to_vector())
}

pub fn map_from_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos - rect.min().to_vector())
  // todo: should effect by transform widget.
}

pub(crate) fn map_to_global(
  id: WidgetId,
  pos: Point,
  tree: &WidgetTree,
  store: &LayoutStore,
) -> Point {
  id.ancestors(tree)
    .fold(pos, |pos, p| map_to_parent(p, pos, store))
}

pub(crate) fn map_from_global(
  id: WidgetId,
  pos: Point,
  tree: &WidgetTree,
  store: &LayoutStore,
) -> Point {
  let stack = id.ancestors(tree).collect::<Vec<_>>();
  stack
    .iter()
    .rev()
    .fold(pos, |pos, p| map_from_parent(*p, pos, store))
}

impl<'a> WidgetCtxImpl for (WidgetId, &'a Context) {
  fn id(&self) -> WidgetId { self.0 }

  fn widget_tree(&self) -> &WidgetTree { &self.1.widget_tree }

  fn layout_store(&self) -> &LayoutStore { &self.1.layout_store }
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;

  fn widget_tree(&self) -> &WidgetTree;

  fn layout_store(&self) -> &LayoutStore;
}

impl<T: WidgetCtxImpl> WidgetCtx for T {
  #[inline]
  fn single_child(&self) -> Option<WidgetId> { self.id().single_child(self.widget_tree()) }

  #[inline]
  fn box_rect(&self) -> Option<Rect> { self.widget_box_rect(self.id()) }

  #[inline]
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self.layout_store().layout_box_rect(wid)
  }

  #[inline]
  fn map_to_global(&self, pos: Point) -> Point {
    map_to_global(self.id(), pos, self.widget_tree(), self.layout_store())
  }

  #[inline]
  fn map_from_global(&self, pos: Point) -> Point {
    map_from_global(self.id(), pos, self.widget_tree(), self.layout_store())
  }

  #[inline]
  fn map_to_parent(&self, pos: Point) -> Point {
    map_to_parent(self.id(), pos, self.layout_store())
  }

  #[inline]
  fn map_from_parent(&self, pos: Point) -> Point {
    map_from_parent(self.id(), pos, self.layout_store())
  }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    map_from_global(w, global, self.widget_tree(), self.layout_store())
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = map_to_global(w, pos, self.widget_tree(), self.layout_store());
    self.map_from_global(global)
  }

  #[inline]
  fn query_type<W: 'static>(&self, id: WidgetId) -> Option<&W> {
    id.assert_get(self.widget_tree())
      .query_first_type(QueryOrder::OutsideFirst)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;

  #[test]
  fn map_self_eq_self() {
    let w = widget! {
      declare SizedBox {
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
