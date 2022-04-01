use painter::{Point, Size};
use text::shaper::TextShaper;
use text::TextReorder;

use super::{WidgetCtx, WidgetCtxImpl};
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
}

impl<'a> LayoutCtx<'a> {
  #[inline]
  pub fn text_shaper(&self) -> &TextShaper { &self.shaper }

  #[inline]
  pub fn text_reorder(&self) -> &TextReorder { &self.text_reorder }

  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    let id = child.render_widget(self.tree).expect("must have");
    self.layout_store.layout_box_rect_mut(id).origin = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you
  /// know what you are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    let id = child.render_widget(self.tree).expect("must have");
    self.layout_store.layout_box_rect_mut(id).size = size;
  }

  /// Do the work of computing the layout for render child, and return its size
  /// it should have. Should called from parent.
  pub fn perform_render_child_layout(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    self
      .layout_store
      .perform_layout(child, clamp, self.tree, self.shaper, self.text_reorder)
  }

  /// Return a tuple of [`LayoutCtx`]! and an iterator of self children, notice
  /// the element of iterator is not its child if it's a combination Widget, but
  /// instead of down to a render widget when precess on child.
  #[inline]
  pub fn split_render_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    self.split_render_children_by(self.id)
  }

  /// Return a tuple of [`LayoutCtx`]! and an iterator of `id`'s children.
  /// Notice the element of iterator is not its child if it's a combination
  /// Widget, but instead of down to a render widget when precess on child.
  pub fn split_render_children_by(
    &mut self,
    id: WidgetId,
  ) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    let children = id
      .children(self.tree)
      .map(|c| c.render_widget(self.tree).unwrap());
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

  /// Return the single render child of `widget`, panic if have more than once
  /// child.
  pub fn single_render_child(&self) -> Option<WidgetId> {
    self
      .single_child()
      .and_then(|id| id.render_widget(self.tree))
  }
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &WidgetTree { self.tree }

  fn layout_store(&self) -> &crate::prelude::LayoutStore { self.layout_store }
}
