use std::{
  cell::{Ref, RefCell},
  sync::Arc,
};

use crate::{
  prelude::*,
  text::{TextBuffer, TextByteIndex, single_style_paragraph_style, single_style_span_style},
};

pub type TextValue = PipeValue<CowArc<str>>;

const ELLIPSIS: &str = "\u{2026}";
const TEXT_FIT_TOLERANCE: f32 = 0.1;

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
  layout: RefCell<Option<Arc<ParagraphLayout>>>,
}

fn text_layout(
  text: CowArc<str>, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> Arc<ParagraphLayout> {
  if text_style.overflow == TextOverflow::Ellipsis {
    return ellipsis_text_layout(text, text_style, text_align, bounds);
  }

  paragraph_layout_for_text(text, text_style, text_align, bounds)
}

fn paragraph_layout_for_text(
  text: CowArc<str>, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> Arc<ParagraphLayout> {
  let paragraph_style = single_style_paragraph_style(text_style, text_align);
  let paragraph = AppCtx::text_services()
    .paragraph(AttributedText::styled(text.to_string(), single_style_span_style(text_style)));
  paragraph.layout(text_style, &paragraph_style, bounds)
}

fn ellipsis_text_layout(
  text: CowArc<str>, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> Arc<ParagraphLayout> {
  let full_layout = paragraph_layout_for_text(text.clone(), text_style, TextAlign::Start, bounds);
  if text.is_empty()
    || !bounds.width.is_finite()
    || full_layout.size().width <= bounds.width + TEXT_FIT_TOLERANCE
  {
    return if text_align == TextAlign::Start {
      full_layout
    } else {
      paragraph_layout_for_text(text, text_style, text_align, bounds)
    };
  }

  let ellipsis_layout = paragraph_layout_for_text(ELLIPSIS.into(), text_style, text_align, bounds);
  let ellipsis_width = ellipsis_layout.size().width;
  if ellipsis_width > bounds.width + TEXT_FIT_TOLERANCE {
    return paragraph_layout_for_text("".into(), text_style, text_align, bounds);
  }

  ellipsized_layout(
    &text,
    &full_layout,
    ellipsis_layout,
    bounds.width - ellipsis_width,
    text_style,
    text_align,
    bounds,
  )
}

fn ellipsized_layout(
  text: &CowArc<str>, full_layout: &Arc<ParagraphLayout>, ellipsis_layout: Arc<ParagraphLayout>,
  available_width: f32, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> Arc<ParagraphLayout> {
  let caret_y = full_layout
    .caret_rect(Caret::default())
    .center()
    .y;
  let mut boundary = full_layout
    .hit_test_point(Point::new(available_width.max(0.), caret_y))
    .caret
    .byte
    .0
    .min(text.len());

  loop {
    if boundary == 0 {
      return ellipsis_layout;
    }

    let layout = paragraph_layout_for_text(
      ellipsis_candidate(text.as_ref(), boundary),
      text_style,
      text_align,
      bounds,
    );
    if layout.size().width <= bounds.width + TEXT_FIT_TOLERANCE {
      return layout;
    }

    let next = text
      .prev_grapheme_boundary(TextByteIndex(boundary))
      .0;
    if next >= boundary {
      return ellipsis_layout;
    }
    boundary = next;
  }
}

fn ellipsis_candidate(text: &str, boundary: usize) -> CowArc<str> {
  let prefix = text[..boundary].trim_end();
  if prefix.is_empty() { ELLIPSIS.into() } else { format!("{prefix}{ELLIPSIS}").into() }
}

impl Render for Text {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    let text_align = Provider::of::<TextAlign>(ctx)
      .map(|align| *align)
      .unwrap_or_default();
    let layout = text_layout(self.text.clone(), &style, text_align, clamp.max);
    let size = layout.size();
    *self.layout.borrow_mut() = Some(layout);
    clamp.clamp(size)
  }

  #[inline]
  fn size_affected_by_child(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let style = Provider::of::<PaintingStyle>(ctx).map(|p| p.clone());
    let Some(layout) = self.layout.borrow().clone() else {
      return;
    };
    let brush = ctx.painter().fill_brush().clone();
    if !brush.is_visible() {
      return;
    }

    let payload = Resource::new(layout.draw_payload().clone());
    let rect = layout.draw_payload().bounds;
    let _ = style;
    ctx.painter().draw_text_payload(payload, rect);
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("text") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> serde_json::Value {
    serde_json::json!({
      "text": *self.text
    })
  }
}

impl Text {
  pub fn new(text: impl Into<CowArc<str>>) -> Self {
    Self { text: text.into(), layout: Default::default() }
  }

  pub fn layout(&self) -> Option<Ref<'_, Arc<ParagraphLayout>>> {
    Ref::filter_map(self.layout.borrow(), |v| v.as_ref()).ok()
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

  fn dejavu_face() -> FontFace {
    FontFace { families: Box::new([FontFamily::Name("DejaVu Sans".into())]), ..Default::default() }
  }

  fn register_test_font() {
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = AppCtx::text_services().register_font_file(std::path::Path::new(&path));
  }

  fn test_text_style(overflow: TextOverflow) -> TextStyle {
    TextStyle {
      font_size: 16.,
      font_face: dejavu_face(),
      letter_space: 0.,
      line_height: crate::text::LineHeight::Px(16.),
      overflow,
    }
  }

  fn last_text_command(frame: Frame) -> TextCommand {
    frame
      .commands
      .into_iter()
      .find_map(|cmd| match cmd {
        PaintCommand::Text(text) => Some(text),
        _ => None,
      })
      .expect("expected a text command")
  }

  fn glyph_count(cmd: &TextCommand) -> usize {
    cmd
      .payload
      .runs
      .iter()
      .map(|run| run.glyphs.len())
      .sum()
  }

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

  #[test]
  fn ellipsis_overflow_fits_within_constrained_width() {
    reset_test_env!();
    register_test_font();

    let text = "Hello ribir ellipsis";
    let max_width = 60.;
    let wnd_size = Size::new(200., 40.);

    let mut overflow_wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(max_width, 20.),
          @Text {
            text,
            text_style: test_text_style(TextOverflow::Overflow),
            foreground: Color::WHITE,
          }
        }
      },
      wnd_size,
    );
    overflow_wnd.draw_frame();
    let overflow = last_text_command(
      overflow_wnd
        .take_last_frame()
        .expect("expected a frame"),
    );

    let mut ellipsis_wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(max_width, 20.),
          @Text {
            text,
            text_style: test_text_style(TextOverflow::Ellipsis),
            foreground: Color::WHITE,
          }
        }
      },
      wnd_size,
    );
    ellipsis_wnd.draw_frame();
    let ellipsis = last_text_command(
      ellipsis_wnd
        .take_last_frame()
        .expect("expected a frame"),
    );

    assert!(overflow.paint_bounds.width() > max_width + super::TEXT_FIT_TOLERANCE);
    assert!(ellipsis.paint_bounds.width() <= max_width + super::TEXT_FIT_TOLERANCE);
    assert!(glyph_count(&ellipsis) < glyph_count(&overflow));
  }
}
