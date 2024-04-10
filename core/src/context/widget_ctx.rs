use std::rc::Rc;

use ribir_geom::{Point, Rect, Size};

use crate::{
  prelude::AppCtx,
  widget::{BoxClamp, WidgetTree},
  widget_tree::WidgetId,
  window::{Window, WindowId},
};

/// common action for all context of widget.
pub trait WidgetCtx {
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
  /// Return if `widget` have child.
  fn has_child(&self) -> bool { self.first_child().is_some() }
  /// Return the first child of widget.
  fn first_child(&self) -> Option<WidgetId>;
  /// Return the box rect of the single child of widget.
  /// # Panic
  /// panic if widget have multi child.
  fn single_child_box(&self) -> Option<Rect>;
  /// Return the widget box rect.
  fn box_rect(&self) -> Option<Rect>;
  /// Return the widget box size.
  fn box_size(&self) -> Option<Size>;
  /// Return the widget box lef-top position .
  fn box_pos(&self) -> Option<Point>;
  /// Return the clamp of the widget that used in last layout.
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
  /// Query type on the widget back of this context, and call the callback if it
  /// found. Return the callback's return value.
  fn query_type<W: 'static, R>(&self, callback: impl FnOnce(&W) -> R) -> Option<R>;
  /// Query type on the widget back of the `id`, and call the callback if it
  /// found. Return the callback's return value.
  fn query_widget_type<W: 'static, R>(
    &self, id: WidgetId, callback: impl FnOnce(&W) -> R,
  ) -> Option<R>;
  /// Get the window of this context, yous should not store the window, store
  /// its id instead.
  fn window(&self) -> Rc<Window>;
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;

  // todo: return sc instead of rc
  fn current_wnd(&self) -> Rc<Window>;

  #[inline]
  fn with_tree<F: FnOnce(&WidgetTree) -> R, R>(&self, f: F) -> R {
    f(&self.current_wnd().widget_tree.borrow())
  }
}

impl<T: WidgetCtxImpl> WidgetCtx for T {
  #[inline]
  fn parent(&self) -> Option<WidgetId> { self.with_tree(|tree| self.id().parent(&tree.arena)) }

  #[inline]
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId> {
    self.with_tree(|tree| w.parent(&tree.arena))
  }

  #[inline]
  fn single_child(&self) -> Option<WidgetId> {
    self.with_tree(|tree| self.id().single_child(&tree.arena))
  }

  #[inline]
  fn first_child(&self) -> Option<WidgetId> {
    self.with_tree(|tree| self.id().first_child(&tree.arena))
  }

  #[inline]
  fn box_rect(&self) -> Option<Rect> { self.widget_box_rect(self.id()) }

  #[inline]
  fn box_pos(&self) -> Option<Point> {
    self.with_tree(|tree| {
      tree
        .store
        .layout_info(self.id())
        .map(|info| info.pos)
    })
  }

  #[inline]
  fn box_size(&self) -> Option<Size> { self.widget_box_size(self.id()) }

  fn layout_clamp(&self) -> Option<BoxClamp> {
    self.with_tree(|tree| {
      tree
        .store
        .layout_info(self.id())
        .map(|info| info.clamp)
    })
  }

  fn single_child_box(&self) -> Option<Rect> {
    self
      .single_child()
      .and_then(|c| self.widget_box_rect(c))
  }

  fn widget_box_size(&self, wid: WidgetId) -> Option<Size> {
    self.with_tree(|tree| {
      tree
        .store
        .layout_info(wid)
        .and_then(|info| info.size)
    })
  }

  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self.with_tree(|tree| {
      tree
        .store
        .layout_info(wid)
        .and_then(|info| info.size.map(|size| Rect::new(info.pos, size)))
    })
  }

  fn map_to_global(&self, pos: Point) -> Point {
    self.with_tree(|tree| {
      tree
        .store
        .map_to_global(pos, self.id(), &tree.arena)
    })
  }

  fn map_from_global(&self, pos: Point) -> Point {
    self.with_tree(|tree| {
      tree
        .store
        .map_from_global(pos, self.id(), &tree.arena)
    })
  }

  fn map_to_parent(&self, pos: Point) -> Point {
    self.with_tree(|tree| {
      tree
        .store
        .map_to_parent(self.id(), pos, &tree.arena)
    })
  }

  fn map_from_parent(&self, pos: Point) -> Point {
    self.with_tree(|tree| {
      tree
        .store
        .map_from_parent(self.id(), pos, &tree.arena)
    })
  }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    self.with_tree(|tree| tree.store.map_from_global(global, w, &tree.arena))
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.with_tree(|tree| tree.store.map_to_global(pos, w, &tree.arena));
    self.map_from_global(global)
  }

  #[inline]
  fn query_type<W: 'static, R>(&self, callback: impl FnOnce(&W) -> R) -> Option<R> {
    self.query_widget_type(self.id(), callback)
  }

  fn query_widget_type<W: 'static, R>(
    &self, id: WidgetId, callback: impl FnOnce(&W) -> R,
  ) -> Option<R> {
    self.with_tree(|tree| {
      id.assert_get(&tree.arena)
        .query_most_outside(callback)
    })
  }

  fn window(&self) -> Rc<Window> { self.current_wnd() }
}

macro_rules! define_widget_context {
  (
    $(#[$outer:meta])*
    $name: ident $(, $extra_name: ident: $extra_ty: ty)*
  ) => {
    $(#[$outer])*
    pub struct $name {
      pub(crate) id: WidgetId,
      pub(crate) wnd_id: WindowId,
      $(pub(crate) $extra_name: $extra_ty,)*
    }

    impl WidgetCtxImpl for $name {
      #[inline]
      fn id(&self) -> WidgetId { self.id }

      #[inline]
      fn current_wnd(&self) -> Rc<Window> { AppCtx::get_window_assert(self.wnd_id) }
    }
  };
}
pub(crate) use define_widget_context;

define_widget_context!(HitTestCtx);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    prelude::*,
    test_helper::{MockBox, TestWindow},
  };

  define_widget_context!(TestCtx);

  #[test]
  fn map_self_eq_self() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        margin: EdgeInsets::all(2.),
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    let tree = &wnd.widget_tree.borrow();
    let root = tree.root();
    let pos = Point::zero();
    let child = root.single_child(&tree.arena).unwrap();

    let w_ctx = TestCtx { id: child, wnd_id: wnd.id() };
    assert_eq!(w_ctx.map_from(pos, child), pos);
    assert_eq!(w_ctx.map_to(pos, child), pos);
  }

  #[test]
  fn map_transform_test() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = fn_widget! {
      @MockBox {
        size: Size::new(100., 100.),
        @MockBox {
          transform: Transform::scale(0.5, 0.5),
          anchor: Anchor::left_top(30., 30.),
          size: Size::new(40., 40.)
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let root = wnd.widget_tree.borrow().root();
    let child = get_single_child_by_depth(root, &wnd.widget_tree.borrow().arena, 3);
    let w_ctx = TestCtx { id: root, wnd_id: wnd.id() };
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
