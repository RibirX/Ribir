use crate::prelude::*;
use crate::render::render_ctx::*;
use crate::render::render_tree::*;
use crate::render::*;

/// Just a stupid implement for develope the framework.
#[stateful]
#[derive(Debug, Declare, Clone)]
pub struct Text {
  pub text: String,
}

impl RenderWidget for Text {
  type RO = Self;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone() }

  #[inline]
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    if self.text != object.text {
      object.text = self.text.clone();
      ctx.mark_needs_layout();
    }
  }
}

impl RenderObject for Text {
  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let rc = ctx.mesure_text(&self.text);
    clamp.clamp(rc.size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    let painter = ctx.painter();
    painter.fill_text(&self.text, None);
  }
}
