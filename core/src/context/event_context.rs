use crate::prelude::WidgetId;
use winit::{event::ModifiersState, window::CursorIcon};

use super::{Context, WidgetCtxImpl};

pub struct EventCtx<'a> {
  id: WidgetId,
  ctx: &'a Context,
}

impl<'a> EventCtx<'a> {
  #[inline]
  pub(crate) fn new(id: WidgetId, ctx: &'a Context) -> Self { Self { id, ctx } }

  #[inline]
  pub fn set_cursor(&mut self, cursor: CursorIcon) { self.ctx.cursor.set(Some(cursor)); }
  #[inline]
  pub fn updated_cursor(&self) -> Option<CursorIcon> { self.ctx.cursor.get() }

  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.ctx.modifiers }
}

impl<'a> WidgetCtxImpl for EventCtx<'a> {
  fn id(&self) -> WidgetId { self.id }

  fn widget_tree(&self) -> &crate::prelude::widget_tree::WidgetTree { &self.ctx.widget_tree }

  fn layout_store(&self) -> &crate::prelude::LayoutStore { &self.ctx.layout_store }
}
