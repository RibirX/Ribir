use crate::{
  prelude::{Painter, WidgetId},
  widget::{LayoutStore, TreeArena},
};

use super::{define_widget_context, AppContext, WidgetCtxImpl};

define_widget_context!(PaintingCtx, painter: &'a mut Painter);

impl<'a> PaintingCtx<'a> {
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }
}
