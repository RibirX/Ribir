use crate::{
  prelude::{Painter, WidgetId},
  window::Window,
};
use std::cell::RefMut;

use super::{define_widget_context, WidgetCtxImpl};

define_widget_context!(PaintingCtx, painter: RefMut<'a, Painter>);

impl<'a> PaintingCtx<'a> {
  pub fn new(id: WidgetId, wnd: &'a Window) -> Self {
    let painter = wnd.painter.borrow_mut();
    Self { id, wnd, painter }
  }
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { &mut self.painter }
}
