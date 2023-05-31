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
  #[declare(default = Palette::of(ctx).on_surface_variant(), convert=into)]
  pub foreground: Brush,
  #[declare(default = TypographyTheme::of(ctx).body_medium.text.clone())]
  pub text_style: CowArc<TextStyle>,
  #[declare(default)]
  pub path_style: PathPaintStyle,
  #[declare(default)]
  pub overflow: Overflow,
}

impl Text {
  pub fn text_layout(&self, t_store: &TypographyStore, bound: Size) -> VisualGlyphs {
    let TextStyle {
      font_size,
      letter_space,
      line_height,
      ref font_face,
      ..
    } = *self.text_style;

    let width: Em = Pixel(bound.width.into()).into();
    let height: Em = Pixel(bound.height.into()).into();

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
        overflow: self.overflow,
      },
    )
  }
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let wnd_ctx = ctx.wnd_ctx();
    self
      .text_layout(wnd_ctx.typography_store(), clamp.max)
      .visual_rect()
      .size
      .cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let bounds = ctx.layout_clamp().map(|b| b.max);
    let visual_glyphs = typography_with_text_style(
      ctx.typography_store(),
      self.text.clone(),
      &self.text_style,
      bounds,
      self.overflow,
    );

    let font_size = self.text_style.font_size.into_pixel().value();
    let rc = visual_glyphs.visual_rect();
    let font_db = ctx.typography_store().font_db().clone();
    let painter = ctx.painter();
    visual_glyphs.visual_rect();
    let Some(paint_rect) = painter.rect_in_paint_bounds(&rc) else { return; };
    if !paint_rect.contains_rect(&rc) {
      painter.clip(Path::rect(&rc));
    }
    paint_glyphs(
      painter,
      font_db,
      visual_glyphs.glyph_bounds_in_rect(paint_rect),
      self.foreground.clone(),
      font_size,
      &self.path_style,
    );
  }
}

pub fn typography_with_text_style<T: Into<Substr>>(
  store: &TypographyStore,
  text: T,
  style: &TextStyle,
  bounds: Option<Size>,
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
      text_align: None,
      bounds,
      line_dir: PlaceLineDirection::TopToBottom,
      overflow,
    },
  )
}

impl Query for Text {
  impl_query_self_only!();
}

macro_rules! define_text_with_theme_style {
  ($name: ident, $style: ident) => {
    #[derive(Declare)]
    pub struct $name {
      #[declare(convert=into)]
      pub text: CowArc<str>,
      #[declare(default = Brush::Color(Palette::of(ctx).on_surface_variant()), convert = into)]
      pub foreground: Brush,
      #[declare(default)]
      pub overflow: Overflow,
    }

    impl Compose for $name {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          init ctx => {
            let text_style = TypographyTheme::of(ctx).$style.text.clone();
          }
          states { this: this.into_readonly() }
          Text {
            text: this.text.clone(),
            foreground: this.foreground.clone(),
            text_style,
            overflow: this.overflow,
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
