use std::rc::Rc;

use ribir_geom::Size;

use super::{AppCtx, WidgetCtx, WidgetCtxImpl};
use crate::{
  widget::{BoxClamp, Layouter, WidgetTree},
  widget_tree::WidgetId,
  window::{Window, WindowId},
};

/// A place to compute the render object's layout. Rather than holding  children
/// directly, `Layout` perform layout across `LayoutCtx`. `LayoutCtx` provide
/// method to perform child layout and also provides methods to update
/// descendants position.
pub struct LayoutCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) wnd_id: WindowId,
  /// The widget tree of the window, not borrow it from `wnd` is because a
  /// `LayoutCtx` always in a mutable borrow.
  pub(crate) tree: &'a mut WidgetTree,
}

impl<'a> WidgetCtxImpl for LayoutCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn current_wnd(&self) -> Rc<Window> { AppCtx::get_window_assert(self.wnd_id) }

  fn with_tree<F: FnOnce(&WidgetTree) -> R, R>(&self, f: F) -> R { f(self.tree) }
}

impl<'a> LayoutCtx<'a> {
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
    self
      .first_child()
      .map(|wid| self.new_layouter(wid))
  }

  /// Return the layouter of the first child.
  pub fn single_child_layouter(&mut self) -> Option<Layouter> {
    self
      .single_child()
      .map(|wid| self.new_layouter(wid))
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
    assert_eq!(child.parent(&self.tree.arena), Some(self.id));
    self.tree.store.force_layout(child).is_some()
  }

  pub(crate) fn new_layouter(&mut self, id: WidgetId) -> Layouter {
    let LayoutCtx { wnd_id, tree, .. } = self;
    Layouter::new(id, *wnd_id, false, tree)
  }
}
