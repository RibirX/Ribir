use ribir_core::prelude::*;
use ticker::FrameMsg;

use super::{CaretState, OnlySizedByParent};
use crate::{input::glyphs_helper::GlyphsHelper, layout::Stack};

class_names! {
  #[doc = "Class name for the text high light rect"]
  TEXT_HIGH_LIGHT,
}

#[derive(Declare)]
pub struct TextHighLight {
  pub rects: Vec<Rect>,
}

impl Compose for TextHighLight {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @Stack {
        @ { pipe!{
          $this.rects.clone().into_iter().map(move |rc| {
            @Container {
              class: TEXT_HIGH_LIGHT,
              anchor: Anchor::from_point(rc.origin),
              size: rc.size,
            }
          })
        }}
      }
    }
    .into_widget()
  }
}

pub fn high_light_widget(
  caret: impl StateWatcher<Value = CaretState>, text: impl StateWatcher<Value = Text>,
) -> Widget<'static> {
  fn_widget! {
    let tick_of_layout_ready = BuildCtx::get().window()
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
    @OnlySizedByParent {
      @TextHighLight {
        rects: pipe!((*$caret, $text.text.clone()))
          .value_chain(move |v| v.sample(tick_of_layout_ready).box_it())
          .map(move|_| $text
            .glyphs()
            .map(|glyphs| glyphs.selection(&$caret.select_range()))
            .unwrap_or_default()
          ),
      }
    }
  }
  .into_widget()
}
