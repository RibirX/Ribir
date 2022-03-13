use crate::prelude::*;

/// The text widget display text with a single style.
#[derive(Debug, Declare, Clone, PartialEq)]
pub struct Text {
  pub text: CowRc<str>,
  #[declare(default = "ctx.theme().typography_theme.body1.text.clone()")]
  pub style: TextStyle,
}

impl RenderWidget for Text {
  fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let shaper = ctx.text_shaper();
    let ids = shaper.font_db_mut().select_all_match(&self.style.font_face);
    let glyphs = shaper.shape_text(&self.text, &ids);
    ::text::layout::glyphs_box(&self.text, &glyphs, self.style.font_size, None, 0.).cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().fill_text(self.text.clone()); }
}
