use std::rc::Rc;

use super::{AppCtx, WidgetCtxImpl};
use crate::{
  prelude::{Painter, WidgetId},
  window::{Window, WindowId},
};

pub struct PaintingCtx<'a> {
  pub(crate) id: WidgetId,
  pub(crate) wnd_id: WindowId,
  pub(crate) painter: &'a mut Painter,
}

impl<'a> WidgetCtxImpl for PaintingCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn current_wnd(&self) -> Rc<Window> { AppCtx::get_window_assert(self.wnd_id) }
}

impl<'a> PaintingCtx<'a> {
  pub fn new(id: WidgetId, wnd_id: WindowId, painter: &'a mut Painter) -> Self {
    Self { id, wnd_id, painter }
  }
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }
}
