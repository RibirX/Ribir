use crate::prelude::WidgetId;
use winit::window::CursorIcon;

use super::{Context, WidgetCtx};

pub struct EventCtx<'a> {
  id: WidgetId,
  cursor: Option<CursorIcon>,
  ctx: &'a Context,
}

impl<'a> EventCtx<'a> {
  #[inline]
  pub fn new(id: WidgetId, ctx: &'a Context) -> Self { Self { id, ctx, cursor: None } }

  #[inline]
  pub fn set_cursor(&mut self, cursor: CursorIcon) { self.cursor = Some(cursor); }
  #[inline]
  pub fn updated_cursor(&self) -> Option<CursorIcon> { self.cursor }
}

impl<'a> WidgetCtx<'a> for EventCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn context(&self) -> &Context { &self.ctx }
}
