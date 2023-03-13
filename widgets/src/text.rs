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
  #[declare(default = Brush::Color(Palette::of(ctx).on_surface_variant()))]
  pub foreground: Brush,
  #[declare(default = TypographyTheme::of(ctx).body_medium.text.clone())]
  pub style: CowArc<TextStyle>,
  #[declare(default)]
  pub path_paint_style: PathPaintStyle,
}

impl Text {
  pub fn text_layout(
    text: &CowArc<str>,
    style: &CowArc<TextStyle>,
    t_store: &TypographyStore,
    bound: BoxClamp,
  ) -> VisualGlyphs {
    let TextStyle {
      font_size,
      letter_space,
      line_height,
      ref font_face,
      ..
    } = **style;

    let width: Em = Pixel(bound.max.width.into()).into();
    let height: Em = Pixel(bound.max.height.into()).into();

    t_store.typography(
      text.substr(..),
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
    let wnd_ctx = ctx.wnd_ctx();
    Text::text_layout(&self.text, &self.style, wnd_ctx.typography_store(), clamp)
      .visual_rect()
      .size
      .cast_unit()
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let rect = ctx.box_rect().unwrap();
    let painter = ctx.painter();
    let TextStyle {
      font_size,
      font_face,
      letter_space,
      line_height,
    } = &*self.style;
    painter
      .set_font(font_face.clone())
      .set_font_size(*font_size);
    if let Some(letter_space) = letter_space {
      painter.set_letter_space(*letter_space);
    }
    if let Some(line_height) = line_height {
      painter.set_text_line_height(*line_height);
    }

    let text = self.text.substr(..);
    let bounds = Some(rect.size);
    match self.path_paint_style {
      PathPaintStyle::Fill => {
        painter.fill_text(text, bounds);
      }
      PathPaintStyle::Stroke(stroke) => {
        painter.stroke_text(text, bounds);
      }
    }
  }
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
      #[declare(default = Brush::Color(Palette::of(ctx).on_surface_variant()))]
      pub foreground: Brush,
    }

    impl Compose for $name {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          init ctx => { let style = TypographyTheme::of(ctx).$style.text.clone(); }
          states { this: this.into_readonly() }
          Text {
            text: this.text.clone(),
            foreground: this.foreground.clone(),
            style: style,
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
