use std::cell::RefCell;
use std::rc::Rc;

use super::{define_widget_context, WidgetCtxImpl, WindowCtx};
use crate::clipboard::Clipboard;
use crate::context::widget_context::WidgetContext;
use crate::{
  events::dispatcher::DispatchInfo,
  widget::{LayoutStore, TreeArena},
  widget_tree::WidgetId,
};
use ribir_geom::Point;
use winit::{event::ModifiersState, window::CursorIcon};

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

  pub fn set_ime_pos(&self, pos: Point) {
    let wnd_ctx = WidgetContext::wnd_ctx(self);
    let pos = self.map_to_global(pos);
    wnd_ctx.set_ime_pos(pos);
  }

  pub fn clipboard(&self) -> Rc<RefCell<dyn Clipboard>> { self.wnd_ctx.app_ctx.clipboard.clone() }
}
