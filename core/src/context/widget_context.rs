use crate::{
  prelude::QueryOrder,
  widget::{BoxClamp, LayoutInfo, LayoutStore, TreeArena},
  widget_tree::WidgetId,
};
use ribir_geom::{Point, Rect, Size};

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
  /// Return the widget box size of the widget of the context.
  fn box_size(&self) -> Option<Size>;
  /// layout clamp
  fn layout_clamp(&self) -> Option<BoxClamp>;
  /// Return the box size of the widget `wid`.
  fn widget_box_size(&self, wid: WidgetId) -> Option<Size>;
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
  fn box_size(&self) -> Option<Size> { self.widget_box_size(self.id()) }

  #[inline]
  fn layout_clamp(&self) -> Option<BoxClamp> {
    self
      .layout_store()
      .layout_info(self.id())
      .map(|info| info.clamp)
  }

  #[inline]
  fn single_child_box(&self) -> Option<Rect> {
    self.single_child().and_then(|c| self.widget_box_rect(c))
  }

  #[inline]
  fn widget_box_size(&self, wid: WidgetId) -> Option<Size> {
    self
      .layout_store()
      .layout_info(wid)
      .and_then(|info| info.size)
  }

  #[inline]
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self
      .layout_store()
      .layout_info(wid)
      .and_then(|info| info.size.map(|size| Rect::new(info.pos, size)))
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

  pub fn set_ime_pos(&self, pos: Point) {
    let wnd_ctx = self.wnd_ctx();
    let pos = self.map_to_global(pos);
    wnd_ctx.set_ime_pos(pos);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    prelude::*,
    test_helper::{MockBox, TestWindow},
    widget::WidgetTree,
  };

  define_widget_context!(TestCtx);

  #[test]
  fn map_self_eq_self() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = widget! {
      MockBox {
        size: Size::zero(),
        margin: EdgeInsets::all(2.),
      }
    };
    let mut wnd = TestWindow::new(w);
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
    let _guard = unsafe { AppCtx::new_lock_scope() };

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

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
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
