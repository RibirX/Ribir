use ribir_text::TypographyStore;

use crate::{
  prelude::{Painter, WidgetId},
  widget::{LayoutStore, TreeArena},
};

use super::{define_widget_context, WidgetCtxImpl, WindowCtx};

define_widget_context!(PaintingCtx, painter: &'a mut Painter);

impl<'a> PaintingCtx<'a> {
  /// Return the 2d painter to draw 2d things.
  #[inline]
  pub fn painter(&mut self) -> &mut Painter { self.painter }

  #[inline]
  pub fn typography_store(&self) -> &TypographyStore { self.wnd_ctx.typography_store() }
}
