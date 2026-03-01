use ribir_geom::Size;

use super::{WidgetCtx, WidgetCtxImpl};
use crate::{
  prelude::{Point, ProviderCtx},
  widget::{BoxClamp, WidgetTree},
  widget_tree::WidgetId,
};

/// A place to compute the render object's layout.
///
/// Rather than holding children directly, `Layout` perform layout across
/// `MeasureCtx`. `MeasureCtx` provide method to perform child layout and also
/// provides methods to update descendants position.
pub struct MeasureCtx<'a> {
  pub(crate) id: WidgetId,
  /// The widget tree of the window, not borrow it from `wnd` is because a
  /// `MeasureCtx` always in a mutable borrow.
  pub(crate) tree: &'a mut WidgetTree,
  pub(crate) provider_ctx: ProviderCtx,
  pub(crate) laid_out_queue: &'a mut Vec<WidgetId>,
}

pub struct PlaceCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) tree: &'a mut WidgetTree,
  pub(crate) provider_ctx: &'a mut ProviderCtx,
}

impl<'a> WidgetCtxImpl for MeasureCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> WidgetCtxImpl for PlaceCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> MeasureCtx<'a> {
  pub(crate) fn new(
    id: WidgetId, tree: &'a mut WidgetTree, laid_out_queue: &'a mut Vec<WidgetId>,
  ) -> Self {
    let provider_ctx = if let Some(p) = id.parent(tree) {
      ProviderCtx::collect_from(p, tree)
    } else {
      ProviderCtx::default()
    };
    Self { id, tree, provider_ctx, laid_out_queue }
  }

  /// Perform layout of the widget of the context and return its size.
  pub(crate) fn perform_layout(&mut self, clamp: BoxClamp) -> Size {
    self
      .get_calculated_size(self.id, clamp)
      .unwrap_or_else(|| {
        // Safety: the `tree` just use to get the widget of `id`, and `tree2` not drop
        // or modify it during measure.
        let tree2 = unsafe { &*(self.tree as *mut WidgetTree) };

        let id = self.id();
        {
          let info = self.tree.store.layout_info_or_default(id);
          info.clamp = clamp;
        }

        debug_assert!(clamp.min.is_finite());
        let size = id.assert_get(tree2).measure(clamp, self);
        debug_assert!(size.is_finite());
        let info = self.tree.store.layout_info_or_default(id);
        info.size = Some(size);

        {
          let mut layout_ctx =
            PlaceCtx { id, tree: self.tree, provider_ctx: &mut self.provider_ctx };
          layout_ctx.perform_place(size);
        }

        self.provider_ctx.pop_providers_for(id);
        self.laid_out_queue.push(id);

        size
      })
  }

  /// Perform layout of the `child` and return its size.
  pub fn layout_child(&mut self, child: WidgetId, clamp: BoxClamp) -> Size {
    self
      .get_calculated_size(child, clamp)
      .unwrap_or_else(|| {
        let id = std::mem::replace(&mut self.id, child);
        let size = self.perform_layout(clamp);
        self.id = id;

        size
      })
  }

  pub fn clamp(&self) -> BoxClamp {
    self
      .tree
      .store
      .layout_info(self.id())
      .unwrap()
      .clamp
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
  /// MeasureCtx` and the iterator of the children.
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
      .map(|child| self.layout_child(child, clamp))
  }

  /// Quick method to do the work of computing the layout for the single child,
  /// and return its size.
  ///
  /// # Panic
  /// panic if there is not only one child it have.
  pub fn assert_perform_single_child_layout(&mut self, clamp: BoxClamp) -> Size {
    let child = self.assert_single_child();
    self.layout_child(child, clamp)
  }

  /// Clear the child layout information, so the `child` will be force layout
  /// when call `[MeasureCtx::layout_child]!` even if it has layout cache
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
}

impl<'a> PlaceCtx<'a> {
  /// Perform the complete placement flow for a widget:
  /// 1. Reset all children positions to zero
  /// 2. Call `Render::place_children` to let the parent place children
  /// 3. Apply `adjust_position` to all children to finalize their positions
  ///
  /// This ensures that even if the parent doesn't explicitly call
  /// `update_position` for some children, their `adjust_position` will still be
  /// triggered.
  pub(crate) fn perform_place(&mut self, size: Size) {
    // Safety: The widget tree structure is immutable during the layout phase.
    let tree2 = unsafe { &*(self.tree as *mut WidgetTree) };
    let id = self.id;

    // Step 1: Reset all children positions to zero before calling place_children
    for child in id.children(tree2) {
      self.tree.store.layout_info_or_default(child).pos = Point::zero();
    }

    // Step 2: Let the widget place its children
    id.assert_get(tree2).place_children(size, self);

    // Step 3: Apply adjust_position to all children
    // We need to push parent's (id) providers back because they were restored
    // after place_children completed. Child's adjust_position may need to
    // access parent's providers.
    let mut buffer = smallvec::SmallVec::new();
    self
      .provider_ctx
      .push_providers_for(id, tree2, &mut buffer);

    for child in id.children(tree2) {
      // Temporarily set id to child for the adjust_position call
      self.id = child;

      // Safety: we need two mutable accesses to the tree - one through self for
      // the PlaceCtx needed by adjust_position, and one for accessing store.
      // These accesses are to different parts of the tree and don't overlap.
      let store = unsafe { &mut (*(&mut *self.tree as *mut WidgetTree)).store };
      let pos = store.layout_info_or_default(child).pos;
      let pos = child.assert_get(tree2).adjust_position(pos, self);
      store.layout_info_or_default(child).pos = pos;
    }

    // Pop parent's providers and restore original id
    self.provider_ctx.pop_providers_for(id);
    self.id = id;
  }

  /// Place the child at the given position.
  ///
  /// Note: The position will be further adjusted by `adjust_position` after
  /// `place_children` completes.
  #[inline]
  pub fn update_position(&mut self, child: WidgetId, pos: Point) {
    self.tree.store.layout_info_or_default(child).pos = pos;
  }

  /// Return the stored position of the widget.
  #[inline]
  pub fn position(&mut self, child: WidgetId) -> Option<Point> {
    self
      .tree
      .store
      .layout_info(child)
      .map(|info| info.pos)
  }
  #[inline]
  pub fn clamp(&self) -> BoxClamp {
    self
      .tree
      .store
      .layout_info(self.id)
      .unwrap()
      .clamp
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

  pub fn widget_box_size(&self, widget: WidgetId) -> Option<Size> {
    self
      .tree
      .store
      .layout_info(widget)
      .and_then(|info| info.size)
  }
}

impl<'w> AsRef<ProviderCtx> for MeasureCtx<'w> {
  fn as_ref(&self) -> &ProviderCtx { &self.provider_ctx }
}

impl<'w> AsMut<ProviderCtx> for MeasureCtx<'w> {
  fn as_mut(&mut self) -> &mut ProviderCtx { &mut self.provider_ctx }
}

impl<'a> AsRef<ProviderCtx> for PlaceCtx<'a> {
  fn as_ref(&self) -> &ProviderCtx { self.provider_ctx }
}

impl<'a> AsMut<ProviderCtx> for PlaceCtx<'a> {
  fn as_mut(&mut self) -> &mut ProviderCtx { self.provider_ctx }
}
