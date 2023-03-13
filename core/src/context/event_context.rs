use super::{define_widget_context, WidgetCtxImpl, WindowCtx};
use crate::{
  cursor_icon::CursorIcon,
  events::{dispatcher::DispatchInfo, ModifiersState},
  widget::{LayoutStore, TreeArena},
  widget_tree::WidgetId,
};
// use winit::{event::ModifiersState, window::CursorIcon};

define_widget_context!(EventCtx, info: &'a mut DispatchInfo);

impl<'a> EventCtx<'a> {
  /// Set window cursor icon.
  #[inline]
  pub fn set_cursor_icon(&mut self, cursor: CursorIcon) { self.info.set_cursor_icon(cursor) }
  /// Return the cursor icon that will submit to window.
  #[inline]
  pub fn stage_cursor_icon(&self) -> Option<CursorIcon> { self.info.stage_cursor_icon() }

  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.info.modifiers() }
}
