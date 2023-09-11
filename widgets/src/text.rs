use ribir_core::{
  impl_query_self_only,
  prelude::{
    typography::{PlaceLineDirection, TypographyCfg},
    *,
  },
};

/// The text widget display text with a single style.
#[derive(Debug, Declare, Declare2, Clone, PartialEq)]
pub struct Text {
  #[declare(convert=into)]
  pub text: CowArc<str>,
  #[declare(default = Palette::of(ctx).on_surface_variant(), convert=into)]
  pub foreground: Brush,
  #[declare(default = TypographyTheme::of(ctx).body_medium.text.clone())]
  pub text_style: CowArc<TextStyle>,
  #[declare(default)]
  pub path_style: PathPaintStyle,
  #[declare(default)]
  pub overflow: Overflow,
  #[declare(default = TextAlign::Start)]
  pub text_align: TextAlign,
}

impl Text {
  pub fn text_layout(&self, bound: Size) -> VisualGlyphs {
    let TextStyle {
      font_size,
      letter_space,
      line_height,
      ref font_face,
      ..
    } = *self.text_style;

    let width: Em = Pixel(bound.width.into()).into();
    let height: Em = Pixel(bound.height.into()).into();
    AppCtx::typography_store().typography(
      self.text.substr(..),
      font_size,
      font_face,
      TypographyCfg {
        line_height,
        letter_space,
        text_align: self.text_align,
        bounds: (width, height).into(),
        line_dir: PlaceLineDirection::TopToBottom,
        overflow: self.overflow,
      },
    )
  }
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    self.text_layout(clamp.max).visual_rect().size.cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let bounds = ctx.layout_clamp().map(|b| b.max);
    let visual_glyphs = typography_with_text_style(
      AppCtx::typography_store(),
      self.text.clone(),
      &self.text_style,
      bounds,
      self.text_align,
      self.overflow,
    );

    let font_db = AppCtx::font_db().clone();
    let font_size = self.text_style.font_size.into_pixel().value();
    let box_rect = Rect::from_size(ctx.box_size().unwrap());
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

pub fn typography_with_text_style<T: Into<Substr>>(
  store: &TypographyStore,
  text: T,
  style: &TextStyle,
  bounds: Option<Size>,
  text_align: TextAlign,
  overflow: Overflow,
) -> VisualGlyphs {
  let &TextStyle {
    font_size,
    letter_space,
    line_height,
    ref font_face,
    ..
  } = style;

  let bounds = if let Some(b) = bounds {
    let width: Em = Pixel(b.width.into()).into();
    let height: Em = Pixel(b.height.into()).into();
    Size::new(width, height)
  } else {
    let max = Em::absolute(f32::MAX);
    Size::new(max, max)
  };

  store.typography(
    text.into(),
    font_size,
    font_face,
    TypographyCfg {
      line_height,
      letter_space,
      text_align,
      bounds,
      line_dir: PlaceLineDirection::TopToBottom,
      overflow,
    },
  )
}

impl_query_self_only!(Text);

macro_rules! define_text_with_theme_style {
  ($name: ident, $style: ident) => {
    #[derive(Declare, Declare2)]
    pub struct $name {
      #[declare(convert=into)]
      pub text: CowArc<str>,
      #[declare(default = Palette::of(ctx).on_surface_variant(), convert = into)]
      pub foreground: Brush,
      #[declare(default)]
      pub overflow: Overflow,
    }

    impl Compose for $name {
      fn compose(this: State<Self>) -> Widget {
        fn_widget! {
          @Text {
            text: pipe!($this.text.clone()),
            foreground: pipe!($this.foreground.clone()),
            text_style: TypographyTheme::of(ctx).$style.text.clone(),
            overflow: pipe!($this.overflow),
          }
        }
        .into()
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
  use crate::layout::SizedBox;

  use super::*;
  use ribir_core::test_helper::*;
  use ribir_geom::Size;

  #[test]
  fn text_clip() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = widget! {
      SizedBox {
        size: Size::new(50., 45.),
        Text {
          text: "hello world,\rnice to meet you.",
        }
      }
    };
    let wnd = TestWindow::new_with_size(w, Size::new(120., 80.));
    wnd.layout();
  }
}
