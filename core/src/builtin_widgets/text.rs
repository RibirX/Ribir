use std::cell::{Ref, RefCell};

use font_db::GlyphBaseline;
use typography::PlaceLineDirection;

use crate::prelude::*;

pub type TextInit = DeclareInit<CowArc<str>>;
/// The text widget displays text with a single style.
///
/// The `TextStyle` provider is utilized to format the text.
///
/// The `TextAlign` provider is used to align multiline text within the text
/// bounds, with the default alignment being
/// `TextAlign::Start`.
#[derive(Declare)]
pub struct Text {
  pub text: CowArc<str>,
  #[declare(skip)]
  glyphs: RefCell<Option<VisualGlyphs>>,
}

pub fn text_glyph(
  text: Substr, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> VisualGlyphs {
  AppCtx::typography_store()
    .borrow_mut()
    .typography(
      text,
      text_style,
      bounds,
      text_align,
      GlyphBaseline::Middle,
      PlaceLineDirection::TopToBottom,
    )
}

pub fn paint_text(
  painter: &mut Painter, glyphs: &VisualGlyphs, style: PaintingStyle, box_rect: Rect,
) {
  if let Some(rect) = painter.intersection_paint_bounds(&box_rect) {
    if let PaintingStyle::Stroke(options) = style {
      painter
        .set_style(PathStyle::Stroke)
        .set_strokes(options);
    } else {
      painter.set_style(PathStyle::Fill);
    }

    let font_db = AppCtx::font_db().clone();
    painter.draw_glyphs_in_rect(glyphs, rect, &font_db.borrow());
  }
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    let text_align = Provider::of::<TextAlign>(ctx).map_or(TextAlign::Start, |t| *t);
    let glyphs = text_glyph(self.text.substr(..), &style, text_align, clamp.max);

    let size = glyphs.visual_rect().size;
    *self.glyphs.borrow_mut() = Some(glyphs);

    clamp.clamp(size)
  }

  fn visual_box(&self, _: &mut VisualCtx) -> Option<Rect> {
    Some(
      self
        .glyphs
        .borrow()
        .as_ref()
        .map(|info| info.visual_rect())
        .unwrap_or_default(),
    )
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let visual_glyphs = self.glyphs().unwrap();
    let rect = visual_glyphs.visual_rect();
    paint_text(ctx.painter(), &visual_glyphs, style.unwrap_or(PaintingStyle::Fill), rect);
  }
}

impl Text {
  pub fn new<const M: u8>(text: impl Into<CowArc<str>>) -> Self {
    Self { text: text.into(), glyphs: Default::default() }
  }
  pub fn glyphs(&self) -> Option<Ref<VisualGlyphs>> {
    Ref::filter_map(self.glyphs.borrow(), |v| v.as_ref()).ok()
  }
}

macro_rules! define_text_with_theme_style {
  ($name:ident, $style:ident) => {
    #[derive(Declare)]
    pub struct $name {
      pub text: CowArc<str>,
    }

    impl Compose for $name {
      fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
          @Text {
            text: pipe!($this.text.clone()),
            text_style: TypographyTheme::of(BuildCtx::get()).$style.text.clone(),
          }
        }
        .into_widget()
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  const WND_SIZE: Size = Size::new(164., 64.);

  widget_test_suit!(
    text_clip,
    WidgetTester::new(fn_widget! {
      @MockBox {
        size: Size::new(50., 45.),
        clip_boundary: true,
        @Text {
          text: "hello world,\rnice to meet you.",
        }
      }
    })
    .with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(Size::new(50., 45.))
  );

  widget_image_tests!(
    default_text,
    WidgetTester::new(fn_widget! {
      @Text { text: "Hello ribir!"}
    })
    .with_wnd_size(WND_SIZE)
  );

  widget_image_tests!(
    h1,
    WidgetTester::new(fn_widget! {
      @H1 { text: "Hello ribir!" }
    })
    .with_wnd_size(WND_SIZE)
  );

  widget_image_tests!(
    middle_baseline,
    WidgetTester::new(self::column! {
      justify_content: JustifyContent::SpaceBetween,
      @Text {
        text: "Baseline check!",
        font_size: 20.,
        text_line_height: 20.,
        background: Color::RED,
      }
      @Text {
        text: "Text line height check!",
        clip_boundary: true,
        font_size: 20.,
        text_line_height: 40.,
        background: Color::GREEN,
      }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.0001)
  );
}
