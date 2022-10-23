use painter::{Point, Size};

use super::WidgetCtxImpl;
use crate::{
  prelude::BoxClamp,
  widget_tree::{WidgetId, WidgetTree},
};

/// A place to compute the render object's layout. Rather than holding children
/// directly, `Layout` perform layout across `LayoutCtx`. `LayoutCtx`
/// provide method to perform child layout and also provides methods to update
/// descendants position.
pub struct LayoutCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a mut WidgetTree,
  pub(crate) performed: &'a mut Vec<WidgetId>,
}

impl<'a> LayoutCtx<'a> {
  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    self.tree.layout_box_rect_mut(child).origin = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you know what you
  /// are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.tree.layout_box_rect_mut(child).size = size;
  }

  /// Do the work of computing the layout for render child, and return its size
  /// it should have. Should called from parent.
  pub fn perform_child_layout(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    child.perform_layout(clamp, self.tree, self.performed)
  }

  /// Return a tuple of [`LayoutCtx`]! and an iterator of `id`'s children.
  pub fn split_children_for(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let tree = self.tree as *mut WidgetTree;
    // Safety: layout context will never mutable access the inner tree. So the
    // iterator is safe.
    let children = id.children(unsafe { &mut *tree });
    (self, children)
  }

  /// Return a tuple of [`LayoutCtx`]! and an reverse iterator of `id`'s
  /// children.
  pub fn split_rev_children_for(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let tree = self.tree as *mut WidgetTree;
    // Safety: layout context will never mutable access the inner tree. So the
    // iterator is safe.
    let children = id.reverse_children(unsafe { &mut *tree });
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
    assert_eq!(child.parent(self.widget_tree()), Some(self.id));
    self.tree.force_layout(child).is_some()
  }
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &WidgetTree { self.tree }
}
