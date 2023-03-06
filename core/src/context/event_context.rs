use std::{cell::RefCell, rc::Rc};

use super::{define_widget_context, WidgetCtxImpl, WindowCtx};
use crate::{
  events::{dispatcher::DispatchInfo, ModifiersState},
  widget::{LayoutStore, TreeArena},
  widget_tree::WidgetId,
  window::CursorIcon,
};
// use winit::{event::ModifiersState, window::CursorIcon};

define_widget_context!(EventCtx, info: Rc<RefCell<dyn DispatchInfo>>);

impl<'a> EventCtx<'a> {
  /// Set window cursor icon.
  #[inline]
  pub fn set_cursor_icon(&mut self, cursor: CursorIcon) {
    let mut info = self.info.borrow_mut();
    info.set_cursor_icon(cursor)
  }
  /// Return the cursor icon that will submit to window.
  #[inline]
  pub fn stage_cursor_icon(&self) -> Option<CursorIcon> { self.info.borrow().stage_cursor_icon() }

  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.info.borrow().modifiers() }
}
