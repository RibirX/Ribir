use super::{AppContext, WidgetCtxImpl};
use crate::{
  prelude::BoxClamp,
  widget::{DirtySet, LayoutStore, TreeArena},
  widget_tree::WidgetId,
};
use painter::{Point, Size};

/// A place to compute the render object's layout. Rather than holding children
/// directly, `Layout` perform layout across `LayoutCtx`. `LayoutCtx`
/// provide method to perform child layout and also provides methods to update
/// descendants position.
pub struct LayoutCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) arena: &'a mut TreeArena,
  pub(crate) store: &'a mut LayoutStore,
  pub(crate) app_ctx: &'a AppContext,
  pub(crate) dirty_set: &'a DirtySet,
}

impl<'a> LayoutCtx<'a> {
  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    self.store_mut().layout_box_rect_mut(child).origin = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you know what you
  /// are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.store_mut().layout_box_rect_mut(child).size = size;
  }

  // todo: ensure user can access next child after previous child performed
  // layout.

  /// Do the work of computing the layout for render child, and return its size
  /// it should have. Should called from parent.
  pub fn perform_child_layout(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    let Self { arena, store, app_ctx, dirty_set, .. } = self;
    store.perform_widget_layout(child, clamp, arena, app_ctx, dirty_set)
  }

  pub fn has_child(&self) -> bool { self.id.first_child(self.tree_arena()).is_some() }

  /// Return a tuple of [`LayoutCtx`]! and an iterator of `id`'s children.
  pub fn split_children_for(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let arena = self.arena_mut() as *mut TreeArena;
    // Safety: layout context will never mutable access the inner tree. So the
    // iterator is safe.
    let children = id.children(unsafe { &mut *arena });
    (self, children)
  }

  /// Return a tuple of [`LayoutCtx`]! and an reverse iterator of `id`'s
  /// children.
  pub fn split_rev_children_for(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let arena = self.arena_mut() as *mut TreeArena;
    // Safety: layout context will never mutable access the inner tree. So the
    // iterator is safe.
    let children = id.reverse_children(unsafe { &mut *arena });
    (self, children)
  }

  /// Return a tuple of [`LayoutCtx`]! and  an reverse iterator of children, so
  /// you can avoid the lifetime problem when precess on child.
  pub fn split_rev_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    self.split_rev_children_for(self.id)
  }

  /// Return a tuple of [`LayoutCtx`]! and  an iterator of children, so
  /// you can avoid the lifetime problem when precess on child.
  pub fn split_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    self.split_children_for(self.id)
  }

  /// Clear the child layout information, so the `child` will be force layout
  /// when call `[LayoutCtx::perform_child_layout]!` even if it has layout cache
  /// information with same input.
  #[inline]
  pub fn force_child_relayout(&mut self, child: WidgetId) -> bool {
    assert_eq!(child.parent(self.tree_arena()), Some(self.id));
    self.store_mut().force_layout(child).is_some()
  }

  fn store_mut(&mut self) -> &mut LayoutStore { &mut self.store }

  fn arena_mut(&mut self) -> &mut TreeArena { &mut self.arena }
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn tree_arena(&self) -> &TreeArena { self.arena }

  fn layout_store(&self) -> &LayoutStore { self.store }

  fn app_ctx(&self) -> &AppContext { self.app_ctx }
}
