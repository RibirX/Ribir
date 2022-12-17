use crate::{
  prelude::QueryOrder,
  widget::{LayoutInfo, LayoutStore, TreeArena},
  widget_tree::WidgetId,
};

use painter::{Point, Rect};

use super::WindowCtx;

/// common action for all context of widget.
pub trait WidgetContext {
  /// Return parent of widget of this context.
  fn parent(&self) -> Option<WidgetId>;
  /// Return parent of widget `w`.
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId>;
  /// Return the single child of `widget`.
  fn single_child(&self) -> Option<WidgetId>;
  /// Return the single child of `widget`.
  /// # Panic
  /// panic if widget have multi child.
  #[inline]
  fn assert_single_child(&self) -> WidgetId { self.single_child().expect("Must have one child.") }
  /// Return the first child of widget.
  fn first_child(&self) -> Option<WidgetId>;
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

  fn wnd_ctx(&self) -> &WindowCtx;
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;
  fn tree_arena(&self) -> &TreeArena;
  fn layout_store(&self) -> &LayoutStore;
  fn wnd_ctx(&self) -> &WindowCtx;
}

impl<T: WidgetCtxImpl> WidgetContext for T {
  #[inline]
  fn parent(&self) -> Option<WidgetId> { self.id().parent(self.tree_arena()) }

  #[inline]
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId> { w.parent(self.tree_arena()) }

  #[inline]
  fn single_child(&self) -> Option<WidgetId> { self.id().single_child(self.tree_arena()) }

  #[inline]
  fn first_child(&self) -> Option<WidgetId> { self.id().first_child(self.tree_arena()) }

  #[inline]
  fn box_rect(&self) -> Option<Rect> { self.widget_box_rect(self.id()) }

  #[inline]
  fn single_child_box(&self) -> Option<Rect> {
    self.single_child().and_then(|c| self.widget_box_rect(c))
  }

  #[inline]
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self.layout_store().layout_box_rect(wid)
  }

  fn map_to_global(&self, pos: Point) -> Point {
    self
      .layout_store()
      .map_to_global(pos, self.id(), self.tree_arena())
  }

  fn map_from_global(&self, pos: Point) -> Point {
    self
      .layout_store()
      .map_from_global(pos, self.id(), self.tree_arena())
  }

  #[inline]
  fn map_to_parent(&self, pos: Point) -> Point {
    self
      .layout_store()
      .map_to_parent(self.id(), pos, self.tree_arena())
  }

  #[inline]
  fn map_from_parent(&self, pos: Point) -> Point {
    self
      .layout_store()
      .map_from_parent(self.id(), pos, self.tree_arena())
  }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    self
      .layout_store()
      .map_from_global(global, w, self.tree_arena())
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.layout_store().map_to_global(pos, w, self.tree_arena());
    self.map_from_global(global)
  }

  #[inline]
  fn query_widget_type<W: 'static>(&self, id: WidgetId, callback: impl FnOnce(&W)) {
    id.assert_get(self.tree_arena())
      .query_on_first_type(QueryOrder::OutsideFirst, callback);
  }

  fn wnd_ctx(&self) -> &WindowCtx { WidgetCtxImpl::wnd_ctx(self) }
}

macro_rules! define_widget_context {
  ($name: ident $(, $extra_name: ident: $extra_ty: ty)*) => {
    pub struct $name<'a> {
      pub(crate) id: WidgetId,
      pub(crate) arena: &'a TreeArena,
      pub(crate) store: &'a LayoutStore,
      pub(crate) wnd_ctx: &'a WindowCtx,
      $(pub(crate) $extra_name: $extra_ty,)*
    }

    impl<'a> WidgetCtxImpl for $name<'a> {
      #[inline]
      fn id(&self) -> WidgetId { self.id }

      #[inline]
      fn tree_arena(&self) -> &TreeArena { self.arena }

      #[inline]
      fn layout_store(&self) -> &LayoutStore { self.store }

      #[inline]
      fn wnd_ctx(&self) -> &WindowCtx { self.wnd_ctx }
    }
  };
}
pub(crate) use define_widget_context;

define_widget_context!(HitTestCtx);
define_widget_context!(LifeCycleCtx);

impl<'a> LifeCycleCtx<'a> {
  pub fn layout_info(&self) -> Option<&LayoutInfo> { self.layout_store().layout_info(self.id()) }

  pub fn wnd_ctx(&self) -> &WindowCtx { WidgetCtxImpl::wnd_ctx(self) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::*, test::MockBox};

  define_widget_context!(TestCtx);

  #[test]
  fn map_self_eq_self() {
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
    let child = root.single_child(&tree.arena).unwrap();
    let WidgetTree { arena, store, wnd_ctx, .. } = tree;
    let w_ctx = TestCtx { id: child, arena, store, wnd_ctx };
    assert_eq!(w_ctx.map_from(pos, child), pos);
    assert_eq!(w_ctx.map_to(pos, child), pos);
  }

  #[test]
  fn map_transform_test() {
    let w = widget! {
      MockBox {
        size: Size::new(100., 100.),
        MockBox {
          transform: Transform::scale(0.5, 0.5),
          left_anchor: 30.,
          top_anchor: 30.,
          size: Size::new(40., 40.)
        }
      }
    };

    let mut wnd = Window::default_mock(w.into_widget(), Some(Size::new(100., 100.)));
    wnd.draw_frame();

    let tree = &wnd.widget_tree;
    let root = tree.root();
    let child = get_single_child_by_depth(root, &tree.arena, 4);
    let WidgetTree { arena, store, wnd_ctx, .. } = tree;
    let w_ctx = TestCtx { id: root, arena, store, wnd_ctx };
    let from_pos = Point::new(30., 30.);
    assert_eq!(w_ctx.map_from(from_pos, child), Point::new(45., 45.));
    let to_pos = Point::new(50., 50.);
    assert_eq!(w_ctx.map_to(to_pos, child), Point::new(40., 40.));
  }

  fn get_single_child_by_depth(id: WidgetId, tree: &TreeArena, mut depth: u32) -> WidgetId {
    let mut child = id;
    while depth > 0 {
      child = child.single_child(tree).unwrap();
      depth -= 1;
    }
    child
  }
}
