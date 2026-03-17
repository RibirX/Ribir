use std::{
  cell::{Ref, RefCell},
  sync::Arc,
};

use crate::{
  prelude::*,
  text::{single_style_paragraph_style, single_style_span_style},
};

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
  layout: RefCell<Option<Arc<ParagraphLayout>>>,
}

fn text_layout(
  text: CowArc<str>, text_style: &TextStyle, text_align: TextAlign, bounds: Size,
) -> Arc<ParagraphLayout> {
  let paragraph_style = single_style_paragraph_style(text_style, text_align);
  let paragraph = AppCtx::text_services()
    .paragraph(AttributedText::styled(text.to_string(), single_style_span_style(text_style)));
  paragraph.layout(text_style, &paragraph_style, bounds)
}

impl Render for Text {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let style = Provider::of::<TextStyle>(ctx).unwrap();
    let layout = text_layout(self.text.clone(), &style, TextAlign::Start, clamp.max);
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
