use painter::Rect;

use super::Context;
use crate::prelude::{
  widget_tree::{WidgetNode, WidgetTree},
  WidgetId,
};

/// common action for all context of widget.
pub trait WidgetCtx<'a> {
  fn id(&self) -> WidgetId;

  fn context(&self) -> &Context;

  /// Return the single child of `widget`, panic if have more than once child.
  fn single_child(&self) -> Option<WidgetId> {
    let ctx = self.context();
    let id = self.id();
    let w_tree = &ctx.widget_tree;
    assert_eq!(id.first_child(w_tree), id.last_child(w_tree));
    id.first_child(w_tree)
  }

  /// Split an iterator of IDs of this widget of the context and its ancestors
  /// form context to avoid lifetime boring.
  fn split_ancestors(self) -> (Self, Box<dyn Iterator<Item = WidgetId> + 'a>)
  where
    Self: Sized,
  {
    let w_tree = &self.context().widget_tree;
    // Safety: context have no way to change the tree struct
    let w_tree = unsafe { &*(w_tree as *const WidgetTree) };
    let iter = self.id().ancestors(w_tree);
    (self, Box::new(iter))
  }

  /// Return the widget box rect of the widget of the context.
  #[inline]
  fn widget_rect(&self) -> Option<Rect> { self.widget_rect_by_id(self.id()) }

  /// Return the box rect of the widget `wid` point to.
  fn widget_rect_by_id(&self, wid: WidgetId) -> Option<Rect> {
    let ctx = self.context();
    let w_tree = &ctx.widget_tree;
    wid
      .down_nearest_render_widget(w_tree)
      .relative_to_render(w_tree)
      .and_then(|rid| ctx.layout_store.layout_box_rect(rid))
  }

  #[inline]
  fn widget(&self) -> &WidgetNode { self.widget_by_id(self.id()) }

  fn widget_by_id(&self, id: WidgetId) -> &WidgetNode {
    let tree = &self.context().widget_tree;
    id.assert_get(tree)
  }
}
