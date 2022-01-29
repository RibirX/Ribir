use std::any::{Any, TypeId};

use painter::{Point, Rect};

use super::Context;
use crate::prelude::{
  widget_tree::{WidgetNode, WidgetTree},
  LayoutStore, WidgetId,
};

/// common action for all context of widget.
pub trait WidgetCtx {
  /// Return the single child of `widget`, panic if have more than once child.
  fn single_child(&self) -> Option<WidgetId>;

  /// Return the widget box rect of the widget of the context.
  fn box_rect(&self) -> Option<Rect>;

  /// Return the box rect of the widget `wid` point to.
  fn widget_box_rect(&self, wid: WidgetId) -> Option<Rect>;

  fn find_attr<A: 'static>(&self) -> Option<&A>
  where
    Self: Sized;

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
  /// `ancestor`. The `ancestor` must be a ancestor of the calling render
  /// object.
  fn map_to(&self, pos: Point, ancestor: WidgetId) -> Point;

  /// Translates the render object coordinate pos from the coordinate system of
  /// ancestor to this render object coordinate system. The parent must be a
  /// parent of the calling render object.
  fn map_from(&self, pos: Point, ancestor: WidgetId) -> Point;

  /// Return the correspond render widget, if this widget is a layout render
  /// widget return self, otherwise find a nearest layout render widget from its
  /// single descendants.
  fn render_widget(&self) -> Option<WidgetId>;

  /// Returns some reference to the inner value if the widget back of `id` is
  /// type `T`, or `None` if it isn't.
  fn widget_downcast_ref<T: 'static>(&self, id: WidgetId) -> Option<&T>;
}

fn map_to_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  // todo: should effect by transform widget.
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos + rect.min().to_vector())
}

fn map_from_parent(id: WidgetId, pos: Point, store: &LayoutStore) -> Point {
  store
    .layout_box_rect(id)
    .map_or(pos, |rect| pos - rect.min().to_vector())
  // todo: should effect by transform widget.
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
  fn find_attr<A: 'static>(&self) -> Option<&A> {
    self.id().assert_get(self.widget_tree()).find_attr()
  }

  fn map_to_global(&self, pos: Point) -> Point {
    self
      .id()
      .ancestors(self.widget_tree())
      .fold(pos, |pos, id| map_to_parent(id, pos, self.layout_store()))
  }

  fn map_from_global(&self, pos: Point) -> Point {
    self
      .id()
      .ancestors(self.widget_tree())
      .fold(pos, |pos, id| map_from_parent(id, pos, self.layout_store()))
  }

  #[inline]
  fn map_to_parent(&self, pos: Point) -> Point {
    map_to_parent(self.id(), pos, self.layout_store())
  }

  #[inline]
  fn map_from_parent(&self, pos: Point) -> Point {
    map_from_parent(self.id(), pos, self.layout_store())
  }

  fn map_to(&self, pos: Point, ancestor: WidgetId) -> Point {
    self
      .id()
      .ancestors(self.widget_tree())
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| map_from_parent(id, pos, self.layout_store()))
  }

  fn map_from(&self, pos: Point, ancestor: WidgetId) -> Point {
    self
      .id()
      .ancestors(self.widget_tree())
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| map_from_parent(id, pos, self.layout_store()))
  }

  #[inline]
  fn render_widget(&self) -> Option<WidgetId> { self.id().render_widget(self.widget_tree()) }

  fn widget_downcast_ref<W: 'static>(&self, id: WidgetId) -> Option<&W> {
    let type_id = TypeId::of::<W>();

    match id.assert_get(self.widget_tree()) {
      WidgetNode::Combination(c) => c.downcast_to(type_id),
      WidgetNode::Render(r) => r.downcast_to(type_id),
    }
    .and_then(<dyn Any>::downcast_ref)
  }
}
