use crate::mark_layout_assign;
use crate::prelude::*;
use crate::render::render_tree::*;
use crate::render::*;

/// The text widget display text with a single style.
#[derive(Debug, Declare, Clone, PartialEq)]
pub struct Text {
  #[declare(convert(into))]
  pub text: CowRc<str>,
  pub style: TextStyle,
}

impl RenderWidget for Text {
  type RO = Self;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone() }

  #[inline]
  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    let obj_style = &mut object.style;
    let style = &self.style;
    mark_layout_assign!(obj_style.font_size, style.font_size, ctx);
    mark_layout_assign!(obj_style.font_face, style.font_face, ctx);
    mark_layout_assign!(obj_style.letter_space, style.letter_space, ctx);
    obj_style.foreground = style.foreground.clone();
  }
}

impl RenderObject for Text {
  #[inline]
  fn perform_layout(&mut self, _: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let shaper = ctx.text_shaper();
    let ids = shaper.font_db_mut().select_all_match(&self.style.font_face);
    let glyphs = shaper.shape_text(&self.text, &ids);
    ::text::layout::glyphs_box(&self.text, &glyphs, self.style.font_size, None, 0.).cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, ctx: &mut PaintingCtx<'a>) {
    let painter = ctx.painter();
    painter.fill_text(self.text.clone());
  }
}
