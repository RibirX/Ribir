use ribir_geom::{Point, Size};

use super::{WidgetCtx, WidgetCtxImpl};
use crate::{
  context::VisualCtx,
  prelude::ProviderCtx,
  widget::{BoxClamp, VisualBox, WidgetTree},
  widget_tree::WidgetId,
  window::DelayEvent,
};

/// A place to compute the render object's layout.
///
/// Rather than holding children directly, `Layout` perform layout across
/// `LayoutCtx`. `LayoutCtx` provide method to perform child layout and also
/// provides methods to update descendants position.
pub struct LayoutCtx<'a> {
  pub(crate) id: WidgetId,
  /// The widget tree of the window, not borrow it from `wnd` is because a
  /// `LayoutCtx` always in a mutable borrow.
  pub(crate) tree: &'a mut WidgetTree,
  pub(crate) provider_ctx: ProviderCtx,
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> LayoutCtx<'a> {
  pub(crate) fn new(id: WidgetId, tree: &'a mut WidgetTree) -> Self {
    let provider_ctx = if let Some(p) = id.parent(tree) {
      ProviderCtx::collect_from(p, tree)
    } else {
      ProviderCtx::default()
    };
    Self { id, tree, provider_ctx }
  }

  /// Perform layout of the widget of the context and return its size.
  pub(crate) fn perform_layout(&mut self, clamp: BoxClamp) -> Size {
    // Safety: the `tree` just use to get the widget of `id`, and `tree2` not drop
    // or modify it during perform layout.
    let tree2 = unsafe { &*(self.tree as *mut WidgetTree) };

    let id = self.id();

    debug_assert!(clamp.min.is_finite());
    let size = id.assert_get(tree2).perform_layout(clamp, self);
    debug_assert!(size.is_finite());
    let info = self.tree.store.layout_info_or_default(id);
    info.clamp = clamp;
    info.size = Some(size);

    {
      VisualCtx::from_layout_ctx(self).update_visual_box();
    }

    self.provider_ctx.pop_providers_for(self.id());
    // TODO: Add event after layout phase completion to avoid excessive event
    // notifications.
    // Example: A Flex container may lay out children twice during its layout
    // process, but only needs to emit a single event for its children.
    self
      .window()
      .add_delay_event(DelayEvent::PerformedLayout(id));

    size
  }

  /// Perform layout of the `child` and return its size.
  pub fn perform_child_layout(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    self
      .get_calculated_size(child, clamp)
      .unwrap_or_else(|| {
        // The position needs to be reset, as some parent render widgets may not have
        // set the position.
        self.update_position(child, Point::zero());

        let id = std::mem::replace(&mut self.id, child);
        let size = self.perform_layout(clamp);
        self.id = id;

        size
      })
  }

  /// Adjust the position of the widget where it should be placed relative to
  /// its parent.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    self.tree.store.layout_info_or_default(child).pos = pos;
  }

  /// Return the position of the widget relative to its parent.
  #[inline]
  pub fn position(&mut self, child: WidgetId) -> Option<Point> {
    self
      .tree
      .store
      .layout_info(child)
      .map(|info| info.pos)
  }

  /// Adjust the size of the layout widget. Use this method to directly modify
  /// the size of a widget. In most cases, it is unnecessary to call this
  /// method; using clamp to constrain the child size is typically sufficient.
  /// Only use this method if you are certain of its effects.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.tree.store.layout_info_or_default(child).size = Some(size);
  }

  /// Split a children iterator from the context, returning a tuple of `&mut
  /// LayoutCtx` and the iterator of the children.
  pub fn split_children(&mut self) -> (&mut Self, impl Iterator<Item = WidgetId> + '_) {
    // Safety: The widget tree structure is immutable during the layout phase, so we
    // can safely split an iterator of children from the layout.
    let tree = unsafe { &*(self.tree as *mut WidgetTree) };
    let id = self.id;
    (self, id.children(tree))
  }

  /// Quick method to do the work of computing the layout for the single child,
  /// and return its size it should have.
  ///
  /// # Panic
  /// panic if there are more than one child it have.
  pub fn perform_single_child_layout(&mut self, clamp: BoxClamp) -> Option<Size> {
    self
      .single_child()
      .map(|child| self.perform_child_layout(child, clamp))
  }

  /// Quick method to do the work of computing the layout for the single child,
  /// and return its size.
  ///
  /// # Panic
  /// panic if there is not only one child it have.
  pub fn assert_perform_single_child_layout(&mut self, clamp: BoxClamp) -> Size {
    let child = self.assert_single_child();
    self.perform_child_layout(child, clamp)
  }

  /// Clear the child layout information, so the `child` will be force layout
  /// when call `[LayoutCtx::perform_child_layout]!` even if it has layout cache
  /// information with same input.
  #[inline]
  pub fn force_child_relayout(&mut self, child: WidgetId) -> bool {
    assert_eq!(child.parent(self.tree), Some(self.id));
    self.tree.store.force_layout(child).is_some()
  }

  fn get_calculated_size(&self, child: WidgetId, clamp: BoxClamp) -> Option<Size> {
    let info = self.tree.store.layout_info(child)?;
    if info.clamp == clamp { info.size } else { None }
  }

  pub(crate) fn visual_box(&mut self, id: WidgetId) -> VisualBox {
    let info = self.tree.store.layout_info_or_default(id);
    info.visual_box
  }

  pub(crate) fn update_visual_box(&mut self) -> VisualBox {
    VisualCtx::from_layout_ctx(self).update_visual_box()
  }
}

impl<'w> AsRef<ProviderCtx> for LayoutCtx<'w> {
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl<'w> AsMut<ProviderCtx> for LayoutCtx<'w> {
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}
