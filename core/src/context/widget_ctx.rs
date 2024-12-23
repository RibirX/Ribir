use std::ptr::NonNull;

use ribir_algo::Sc;
use ribir_geom::{Point, Rect, Size};

use crate::{
  query::QueryRef,
  state::WriteRef,
  widget::{BoxClamp, WidgetTree},
  widget_tree::WidgetId,
  window::Window,
};

/// common action for all context of widget.
pub trait WidgetCtx {
  /// This indicates the widget ID represented by the context.
  fn widget_id(&self) -> WidgetId;
  /// Return parent of widget of this context.
  fn parent(&self) -> Option<WidgetId>;
  // Determine if the current widget in the context is an ancestor of `w`.
  fn ancestor_of(&self, w: WidgetId) -> bool;
  // Determine if the current widget in the context is an successor of `w`.
  fn successor_of(&self, w: WidgetId) -> bool;
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
  /// Return the position of the widget that `wid` references.
  fn widget_box_pos(&self, wid: WidgetId) -> Option<Point>;
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
  /// Query all references to the `T` if it is shared within the widget
  /// represented by this context.
  fn query_all_iter<T: 'static>(&self) -> impl DoubleEndedIterator<Item = QueryRef<T>>;
  /// Query a reference to the `T` if it is shared within the widget
  /// represented by this context.
  ///
  /// This method differs from `Provider::of`, as `Provider::of` searches in all
  /// ancestors of the widget, whereas this method only searches within the
  /// current widget.
  fn query<T: 'static>(&self) -> Option<QueryRef<T>>;
  /// Query a write reference to the `T` if a writer of `T` is shared within the
  /// widget represented by this context.
  ///
  /// This method differs from `Provider::write_of`, as `Provider::write_of`
  /// searches in all ancestors of the widget, whereas this method only
  /// searches within the current widget.
  fn query_write<T: 'static>(&self) -> Option<WriteRef<T>>;
  /// Query a reference to the `T` if it is shared within the widget `w`.
  fn query_of_widget<T: 'static>(&self, w: WidgetId) -> Option<QueryRef<T>>;
  // Query a write reference to the `T` if a writer of `T` is shared within the
  // widget `w`.
  fn query_write_of_widget<T: 'static>(&self, w: WidgetId) -> Option<WriteRef<T>>;
  /// Retrieve the window associated with this context.
  fn window(&self) -> Sc<Window>;
}

pub(crate) trait WidgetCtxImpl {
  fn id(&self) -> WidgetId;

  fn tree(&self) -> &WidgetTree;
}

impl<T: WidgetCtxImpl> WidgetCtx for T {
  #[inline]
  fn widget_id(&self) -> WidgetId { self.id() }

  #[inline]
  fn parent(&self) -> Option<WidgetId> { self.id().parent(self.tree()) }

  #[inline]
  fn ancestor_of(&self, w: WidgetId) -> bool { self.id().ancestor_of(w, self.tree()) }

  #[inline]
  fn successor_of(&self, w: WidgetId) -> bool { w.ancestor_of(self.id(), self.tree()) }

  #[inline]
  fn widget_parent(&self, w: WidgetId) -> Option<WidgetId> { w.parent(self.tree()) }

  #[inline]
  fn single_child(&self) -> Option<WidgetId> { self.id().single_child(self.tree()) }

  #[inline]
  fn first_child(&self) -> Option<WidgetId> { self.id().first_child(self.tree()) }

  #[inline]
  fn box_rect(&self) -> Option<Rect> { self.widget_box_rect(self.id()) }

  #[inline]
  fn box_pos(&self) -> Option<Point> { self.widget_box_pos(self.id()) }

  #[inline]
  fn box_size(&self) -> Option<Size> { self.widget_box_size(self.id()) }

  fn layout_clamp(&self) -> Option<BoxClamp> {
    self
      .tree()
      .store
      .layout_info(self.id())
      .map(|info| info.clamp)
  }

  fn single_child_box(&self) -> Option<Rect> {
    self
      .single_child()
      .and_then(|c| self.widget_box_rect(c))
  }

  fn widget_box_size(&self, wid: WidgetId) -> Option<Size> {
    self
      .tree()
      .layout_info(wid)
      .and_then(|info| info.size)
  }

  fn widget_box_pos(&self, wid: WidgetId) -> Option<Point> {
    self.tree().layout_info(wid).map(|info| info.pos)
  }

  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect> {
    self
      .tree()
      .layout_info(wid)
      .and_then(|info| info.size.map(|size| Rect::new(info.pos, size)))
  }

  fn map_to_global(&self, pos: Point) -> Point { self.tree().map_to_global(pos, self.id()) }

  fn map_from_global(&self, pos: Point) -> Point { self.tree().map_from_global(pos, self.id()) }

  fn map_to_parent(&self, pos: Point) -> Point { self.tree().map_to_parent(self.id(), pos) }

  fn map_from_parent(&self, pos: Point) -> Point { self.tree().map_from_parent(self.id(), pos) }

  fn map_to(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.map_to_global(pos);
    self.tree().map_from_global(global, w)
  }

  fn map_from(&self, pos: Point, w: WidgetId) -> Point {
    let global = self.tree().map_to_global(pos, w);
    self.map_from_global(global)
  }

  fn query_all_iter<Q: 'static>(&self) -> impl DoubleEndedIterator<Item = QueryRef<Q>> {
    self.id().query_all_iter(self.tree())
  }

  fn query<Q: 'static>(&self) -> Option<QueryRef<Q>> { self.query_of_widget::<Q>(self.id()) }

  #[inline]
  fn query_write<Q: 'static>(&self) -> Option<WriteRef<Q>> {
    self.query_write_of_widget::<Q>(self.id())
  }

  fn query_of_widget<Q: 'static>(&self, w: WidgetId) -> Option<QueryRef<Q>> {
    w.query_ref::<Q>(self.tree())
  }

  fn query_write_of_widget<Q: 'static>(&self, w: WidgetId) -> Option<WriteRef<Q>> {
    w.query_write(self.tree())
  }

  fn window(&self) -> Sc<Window> { self.tree().window() }
}

macro_rules! define_widget_context {
  (
    $(#[$outer:meta])*
    $name: ident $(, $extra_name: ident: $extra_ty: ty)*
  ) => {
    $(#[$outer])*
    pub struct $name {
      pub(crate) id: WidgetId,
      pub(crate) tree: NonNull<WidgetTree>,
      $(pub(crate) $extra_name: $extra_ty,)*
    }

    impl WidgetCtxImpl for $name {
      #[inline]
      fn id(&self) -> WidgetId { self.id }

      fn tree(&self) -> &WidgetTree {
        unsafe {self.tree.as_ref()}
      }
    }
  };
}
pub(crate) use define_widget_context;

define_widget_context!(HitTestCtx);

impl HitTestCtx {
  pub fn box_hit_test(&self, pos: Point) -> bool {
    self
      .box_rect()
      .is_some_and(|rect| rect.contains(pos))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::*, reset_test_env, test_helper::*};

  define_widget_context!(TestCtx);

  #[test]
  fn map_self_eq_self() {
    reset_test_env!();

    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        margin: EdgeInsets::all(2.),
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    let tree = wnd.tree();
    let root = tree.root();
    let pos = Point::zero();
    let child = root.single_child(tree).unwrap();

    let w_ctx = TestCtx { id: child, tree: wnd.tree };
    assert_eq!(w_ctx.map_from(pos, child), pos);
    assert_eq!(w_ctx.map_to(pos, child), pos);
  }

  #[test]
  fn map_transform_test() {
    reset_test_env!();

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

    let root = wnd.tree().root();
    let child = get_single_child_by_depth(root, wnd.tree(), 2);
    let w_ctx = TestCtx { id: root, tree: wnd.tree };
    let from_pos = Point::new(30., 30.);
    assert_eq!(w_ctx.map_from(from_pos, child), Point::new(45., 45.));
    let to_pos = Point::new(50., 50.);
    assert_eq!(w_ctx.map_to(to_pos, child), Point::new(40., 40.));
  }

  fn get_single_child_by_depth(id: WidgetId, tree: &WidgetTree, mut depth: u32) -> WidgetId {
    let mut child = id;
    while depth > 0 {
      child = child.single_child(tree).unwrap();
      depth -= 1;
    }
    child
  }
}
