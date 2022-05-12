use ::text::typography::{PlaceLineDirection, TypographyCfg};
pub use ::text::{typography::Overflow, *};

use crate::prelude::*;

/// The text widget display text with a single style.
#[derive(Debug, Declare, Clone, PartialEq)]
pub struct Text {
  #[declare(custom_convert)]
  pub text: ArcStr,
  #[declare(default = "ctx.theme().typography_theme.body1.text.clone()")]
  pub style: TextStyle,
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let t_store = ctx.typography_store();
    let TextStyle {
      font_size,
      letter_space,
      line_height,
      ref font_face,
      ..
    } = self.style;

    let width: Em = Pixel(clamp.max.width.into()).into();
    let height: Em = Pixel(clamp.max.width.into()).into();

    let visual_info = t_store.typography(
      self.text.substr(..),
      font_size,
      font_face,
      TypographyCfg {
        line_height,
        letter_space,
        text_align: None,
        bounds: (width, height).into(),
        line_dir: PlaceLineDirection::TopToBottom,
        overflow: Overflow::Clip,
      },
    );
    visual_info.visual_rect().size.cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let rect = ctx.box_rect().unwrap();
    ctx
      .painter()
      .paint_text_with_style(self.text.substr(..), &self.style, Some(rect.size));
  }
}

impl TextBuilder {
  #[inline]
  pub fn text_convert<T: Into<ArcStr>>(text: T) -> ArcStr { text.into() }
}
