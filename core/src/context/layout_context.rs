use std::sync::{Arc, RwLock};

use painter::{Point, Size};
use text::font_db::FontDB;
use text::shaper::TextShaper;
use text::{TextReorder, TypographyStore};

use super::WidgetCtxImpl;
use crate::prelude::widget_tree::WidgetTree;
use crate::prelude::{BoxClamp, LayoutStore, WidgetId};

/// A place to compute the render object's layout. Rather than holding children
/// directly, `Layout` perform layout across `LayoutCtx`. `LayoutCtx`
/// provide method to perform child layout and also provides methods to update
/// descendants position.
pub struct LayoutCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a WidgetTree,
  pub(crate) layout_store: &'a mut LayoutStore,
  pub(crate) shaper: &'a TextShaper,
  pub(crate) text_reorder: &'a TextReorder,
  pub(crate) typography_store: &'a TypographyStore,
  pub(crate) font_db: &'a Arc<RwLock<FontDB>>,
}

impl<'a> LayoutCtx<'a> {
  #[inline]
  pub fn text_shaper(&self) -> &TextShaper { &self.shaper }

  #[inline]
  pub fn text_reorder(&self) -> &TextReorder { &self.text_reorder }

  #[inline]
  pub fn typography_store(&self) -> &TypographyStore { &self.typography_store }

  #[inline]
  pub fn font_db(&self) -> Arc<RwLock<FontDB>> { self.font_db.clone() }

  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    self.layout_store.layout_box_rect_mut(child).origin = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you
  /// know what you are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.layout_store.layout_box_rect_mut(child).size = size;
  }

  /// Do the work of computing the layout for render child, and return its size
  /// it should have. Should called from parent.
  pub fn perform_child_layout(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    self.layout_store.perform_layout(
      child,
      clamp,
      self.tree,
      self.shaper,
      self.text_reorder,
      self.typography_store,
      self.font_db,
    )
  }

  /// Return a tuple of [`LayoutCtx`]! and an iterator of `id`'s children.
  pub fn split_children_by(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let children = id.children(self.tree);
    (self, children)
  }

  /// Return a tuple of [`LayoutCtx`]! and  an reverse iterator of children, so
  /// you can avoid the lifetime problem when precess on child.
  pub fn split_rev_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let iter = self.id.reverse_children(self.tree);
    (self, iter)
  }

  /// Return a tuple of [`LayoutCtx`]! and  an iterator of children, so
  /// you can avoid the lifetime problem when precess on child.
  pub fn split_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let iter = self.id.children(self.tree);
    (self, iter)
  }
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &WidgetTree { self.tree }

  fn layout_store(&self) -> &crate::prelude::LayoutStore { self.layout_store }
}
