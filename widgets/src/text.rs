use ribir_core::{
  impl_query_self_only,
  prelude::{
    typography::{PlaceLineDirection, TypographyCfg},
    *,
  },
};
/// The text widget display text with a single style.
#[derive(Debug, Declare, Clone, PartialEq)]
pub struct Text {
  #[declare(convert=into)]
  pub text: CowArc<str>,
  #[declare(default = TypographyTheme::of(ctx.theme()).body1.text.clone())]
  pub style: TextStyle,
}

impl Text {
  pub fn text_layout(&self, t_store: &TypographyStore, bound: BoxClamp) -> VisualGlyphs {
    let TextStyle {
      font_size,
      letter_space,
      line_height,
      ref font_face,
      ..
    } = self.style;

    let width: Em = Pixel(bound.max.width.into()).into();
    let height: Em = Pixel(bound.max.height.into()).into();

    t_store.typography(
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
    )
  }
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let app_ctx = ctx.app_ctx();
    self
      .text_layout(&app_ctx.typography_store, clamp)
      .visual_rect()
      .size
      .cast_unit()
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

impl Query for Text {
  impl_query_self_only!();
}
