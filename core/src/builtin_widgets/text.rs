use std::cell::{Ref, RefCell};

use font_db::GlyphBaseline;
use typography::PlaceLineDirection;

use crate::prelude::*;

pub type TextValue = PipeValue<CowArc<str>>;

/// A single-style text widget that displays a single string of text.
///
/// This Text will inherit the text style from its parent widget. You can
/// also use the builtin field to set it:
/// 1. `text_style` (see ./text_style.rs) - This is a builtin field of FatObj.
///    You can simply set the `text_style` field to attach a TextStyleWidget to
///    the host widget.
/// 2. `font_size` (set by text_style, see ./text_style.rs)
/// 3. `text_line_height` (set by text_style, see ./text_style.rs)
/// 4. `text_align` (see ./text_align.rs) - This is a builtin field of FatObj.
///    You can simply set the `text_align` field to attach a TextAlignWidget to
///    the host widget.
/// 5. `foreground` (see ./painting_style.rs)
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "Hello Ribir!",
///     foreground: Color::RED,
///   }
/// };
/// ```
#[declare]
pub struct Text {
  /// The text content to display, using copy-on-write semantics for efficient
  /// string handling
  pub text: CowArc<str>,

  /// Cached glyph layout results for the current text and style configuration
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
    let mut glyphs = text_glyph(self.text.substr(..), &style, text_align, clamp.max);
    let mut size = glyphs.visual_rect().size;
    if text_align != TextAlign::Start {
      size.width = clamp.container_width(size.width);
      glyphs.align(Rect::from_size(size));
    }

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
  fn size_affected_by_child(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let visual_glyphs = self.glyphs().unwrap();
    let rect = visual_glyphs.visual_rect();
    paint_text(ctx.painter(), &visual_glyphs, style.unwrap_or(PaintingStyle::Fill), rect);
  }
}

impl Text {
  pub fn new(text: impl Into<CowArc<str>>) -> Self {
    Self { text: text.into(), glyphs: Default::default() }
  }
  pub fn glyphs(&self) -> Option<Ref<'_, VisualGlyphs>> {
    Ref::filter_map(self.glyphs.borrow(), |v| v.as_ref()).ok()
  }
}

macro_rules! define_text_with_theme_style {
  ($name:ident, $style:ident) => {
    #[declare]
    pub struct $name {
      pub text: CowArc<str>,
    }

    impl Compose for $name {
      fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
          @Text {
            text: pipe!($read(this).text.clone()),
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
