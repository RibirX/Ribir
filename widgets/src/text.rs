use ribir_core::prelude::*;

/// The text widget display text with a single style.
#[derive(Debug, Declare, Query, Clone, PartialEq)]
pub struct Text {
  pub text: CowArc<str>,
  #[declare(default = Palette::of(ctx!()).on_surface_variant())]
  pub foreground: Brush,
  #[declare(default = TypographyTheme::of(ctx!()).body_medium.text.clone())]
  pub text_style: CowArc<TextStyle>,
  #[declare(default)]
  pub path_style: PathStyle,
  #[declare(default)]
  pub overflow: Overflow,
  #[declare(default = TextAlign::Start)]
  pub text_align: TextAlign,
}

impl VisualText for Text {
  fn text(&self) -> CowArc<str> { self.text.clone() }
  fn text_style(&self) -> &TextStyle { &self.text_style }
  fn text_align(&self) -> TextAlign { self.text_align }
  fn overflow(&self) -> Overflow { self.overflow }
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    self
      .text_layout(AppCtx::typography_store(), clamp.max)
      .visual_rect()
      .size
      .cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let box_rect = Rect::from_size(ctx.box_size().unwrap());
    if ctx
      .painter()
      .intersection_paint_bounds(&box_rect)
      .is_none()
    {
      return;
    };

    let bounds = ctx.layout_clamp().map(|b| b.max).unwrap();
    let visual_glyphs = self.text_layout(AppCtx::typography_store(), bounds);
    let font_db = AppCtx::font_db().clone();
    let font_size = self.text_style.font_size.into_pixel().value();
    draw_glyphs_in_rect(
      ctx.painter(),
      visual_glyphs,
      box_rect,
      self.foreground.clone(),
      font_size,
      &self.path_style,
      font_db,
    );
  }
}

macro_rules! define_text_with_theme_style {
  ($name:ident, $style:ident) => {
    #[derive(Declare)]
    pub struct $name {
      pub text: CowArc<str>,
      #[declare(default = Palette::of(ctx!()).on_surface_variant())]
      pub foreground: Brush,
      #[declare(default)]
      pub overflow: Overflow,
    }

    impl Compose for $name {
      fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
        fn_widget! {
          @Text {
            text: pipe!($this.text.clone()),
            foreground: pipe!($this.foreground.clone()),
            text_style: TypographyTheme::of(ctx!()).$style.text.clone(),
            overflow: pipe!($this.overflow),
          }
        }
      }
    }
  };
}

define_text_with_theme_style!(H1, headline_large);
define_text_with_theme_style!(H2, headline_medium);
define_text_with_theme_style!(H3, headline_small);
define_text_with_theme_style!(H4, title_large);
define_text_with_theme_style!(H5, title_medium);
define_text_with_theme_style!(H6, title_small);

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;

  use super::*;
  use crate::layout::SizedBox;

  #[test]
  fn text_clip() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = fn_widget! {
      @SizedBox {
        size: Size::new(50., 45.),
        @Text {
          text: "hello world,\rnice to meet you.",
        }
      }
    };
    let wnd = TestWindow::new_with_size(w, Size::new(120., 80.));
    wnd.layout();
  }
}
