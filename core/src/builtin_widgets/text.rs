use std::cell::{Ref, RefCell};

use typography::PlaceLineDirection;

use crate::prelude::*;

pub type TextInit = DeclareInit<CowArc<str>>;
/// The text widget display text with a single style.
#[derive(Declare)]
pub struct Text {
  pub text: CowArc<str>,
  #[declare(default = TextAlign::Start)]
  pub text_align: TextAlign,
  #[declare(default)]
  glyphs: RefCell<Option<VisualGlyphs>>,
}

impl Render for Text {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let style = ctx.text_style();
    let info = AppCtx::typography_store()
      .borrow_mut()
      .typography(
        self.text.substr(..),
        style,
        clamp.max,
        self.text_align,
        PlaceLineDirection::TopToBottom,
      );

    let size = info.visual_rect().size;
    *self.glyphs.borrow_mut() = Some(info);

    clamp.clamp(size)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let box_rect = Rect::from_size(ctx.box_size().unwrap());
    if ctx
      .painter()
      .intersection_paint_bounds(&box_rect)
      .is_none()
    {
      return;
    };

    let visual_glyphs = self.glyphs().unwrap();
    let font_db = AppCtx::font_db().clone();
    ctx
      .painter()
      .draw_glyphs_in_rect(&visual_glyphs, box_rect, &font_db.borrow());
  }
}

impl Text {
  pub fn new<const M: u8>(text: impl Into<CowArc<str>>) -> Self {
    Self { text: text.into(), text_align: TextAlign::Start, glyphs: Default::default() }
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
      @ MockBox {
        size: Size::new(50., 45.),
        @Text {
          text: "hello world,\rnice to meet you.",
        }
      }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025),
    LayoutCase::default().with_size(Size::new(50., 45.))
  );

  widget_image_tests!(
    default_text,
    WidgetTester::new(fn_widget! {
      @Text { text: "Hello ribir!"}
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025)
  );

  widget_image_tests!(
    h1,
    WidgetTester::new(fn_widget! {
      @H1 { text: "Hello ribir!" }
    })
    .with_wnd_size(WND_SIZE)
    .with_comparison(0.000025)
  );
}
