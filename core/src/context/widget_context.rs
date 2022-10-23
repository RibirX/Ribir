use super::AppContext;
use crate::{
  prelude::QueryOrder,
  widget_tree::{WidgetId, WidgetTree},
};
use painter::{Point, Rect};

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
  fn single_child_box(&self) -> Option<Rect>;
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

  fn app_ctx(&self) -> &AppContext;
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;

  fn widget_tree(&self) -> &WidgetTree;
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
  fn single_child_box(&self) -> Option<Rect> {
    self.single_child().and_then(|c| self.widget_box_rect(c))
  }

  #[inline]
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self.widget_tree().layout_box_rect(wid)
  }

  #[inline]
  fn map_to_global(&self, pos: Point) -> Point { self.widget_tree().map_to_global(self.id(), pos) }

  #[inline]
  fn map_from_global(&self, pos: Point) -> Point {
    self.widget_tree().map_from_global(self.id(), pos)
  }

  #[inline]
  fn map_to_parent(&self, pos: Point) -> Point { self.widget_tree().map_to_parent(self.id(), pos) }

  #[inline]
  fn map_from_parent(&self, pos: Point) -> Point {
    self.widget_tree().map_from_parent(self.id(), pos)
  }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    self.widget_tree().map_from_global(w, global)
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.widget_tree().map_to_global(w, pos);
    self.map_from_global(global)
  }

  #[inline]
  fn query_widget_type<W: 'static>(&self, id: WidgetId, callback: impl FnOnce(&W)) {
    id.assert_get(self.widget_tree())
      .query_on_first_type(QueryOrder::OutsideFirst, callback);
  }

  #[inline]
  fn app_ctx(&self) -> &AppContext { self.widget_tree().app_ctx() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::*, test::MockBox};

  #[test]
  fn map_self_eq_self() {
    impl<'a> WidgetCtxImpl for (WidgetId, &'a WidgetTree) {
      fn id(&self) -> WidgetId { self.0 }

      fn widget_tree(&self) -> &WidgetTree { self.1 }
    }

    let w = widget! {
      MockBox {
        size: Size::zero(),
        margin: EdgeInsets::all(2.),
      }
    };
    let mut wnd = Window::default_mock(w.into_widget(), None);
    wnd.draw_frame();

    let tree = &wnd.widget_tree;
    let root = tree.root();
    let pos = Point::zero();
    let child = root.single_child(tree).unwrap();

    let w_ctx = (child, tree);
    assert_eq!(w_ctx.map_from(pos, child), pos);
    assert_eq!(w_ctx.map_to(pos, child), pos);
  }
}
