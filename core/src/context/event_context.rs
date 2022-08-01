use super::WidgetCtxImpl;
use crate::prelude::{dispatcher::DispatchInfo, widget_tree::WidgetTree, WidgetId};
use winit::{event::ModifiersState, window::CursorIcon};

pub struct EventCtx<'a> {
  id: WidgetId,
  tree: &'a WidgetTree,
  info: &'a mut DispatchInfo,
}

impl<'a> EventCtx<'a> {
  #[inline]
  pub(crate) fn new(id: WidgetId, tree: &'a WidgetTree, info: &'a mut DispatchInfo) -> Self {
    Self { id, tree, info }
  }
  /// Set window cursor icon.
  #[inline]
  pub fn set_cursor_icon(&mut self, cursor: CursorIcon) { self.info.set_cursor_icon(cursor) }
  /// Return the cursor icon that will submit to window.
  #[inline]
  pub fn stage_cursor_icon(&self) -> Option<CursorIcon> { self.info.stage_cursor_icon() }

  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.info.modifiers() }
}

impl<'a> WidgetCtxImpl for EventCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &crate::prelude::widget_tree::WidgetTree { self.tree }

  fn context(&self) -> Option<&super::Context> { None }
}
