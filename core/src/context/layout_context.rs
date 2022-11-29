use super::{AppContext, WidgetContext, WidgetCtxImpl};
use crate::{
  widget::{BoxClamp, DirtySet, LayoutStore, Layouter, TreeArena},
  widget_tree::WidgetId,
};
use painter::Size;

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
  /// Return if there is child of this widget.
  #[inline]
  pub fn has_child(&self) -> bool { self.id.first_child(self.tree_arena()).is_some() }

  /// Quick method to do the work of computing the layout for the single child,
  /// and return its size it should have.
  ///
  /// # Panic
  /// panic if there are more than one child it have.
  pub fn perform_single_child_layout(&mut self, clamp: BoxClamp) -> Option<Size> {
    self
      .single_child_layouter()
      .map(|mut l| l.perform_widget_layout(clamp))
  }

  /// Quick method to do the work of computing the layout for the single child,
  /// and return its size.
  ///
  /// # Panic
  /// panic if there is not only one child it have.
  pub fn assert_perform_single_child_layout(&mut self, clamp: BoxClamp) -> Size {
    self
      .assert_single_child_layouter()
      .perform_widget_layout(clamp)
  }

  /// Return the layouter of the first child.
  pub fn first_child_layouter(&mut self) -> Option<Layouter> {
    self.first_child().map(|wid| self.new_layouter(wid))
  }

  /// Return the layouter of the first child.
  pub fn single_child_layouter(&mut self) -> Option<Layouter> {
    self.single_child().map(|wid| self.new_layouter(wid))
  }

  /// Return the layouter of the first child.
  /// # Panic
  /// panic if there is not only one child it have.
  pub fn assert_single_child_layouter(&mut self) -> Layouter {
    let wid = self.assert_single_child();
    self.new_layouter(wid)
  }

  /// Clear the child layout information, so the `child` will be force layout
  /// when call `[LayoutCtx::perform_child_layout]!` even if it has layout cache
  /// information with same input.
  #[inline]
  pub fn force_child_relayout(&mut self, child: WidgetId) -> bool {
    assert_eq!(child.parent(&self.arena), Some(self.id));
    self.store.force_layout(child).is_some()
  }

  fn new_layouter(&mut self, wid: WidgetId) -> Layouter {
    let Self { arena, store, app_ctx, dirty_set, .. } = self;
    Layouter {
      wid,
      arena,
      store,
      app_ctx,
      dirty_set,
    }
  }
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn tree_arena(&self) -> &TreeArena { self.arena }

  fn layout_store(&self) -> &LayoutStore { self.store }

  fn app_ctx(&self) -> &AppContext { self.app_ctx }
}
