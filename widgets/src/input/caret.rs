use ribir_core::prelude::*;
use ticker::FrameMsg;

use crate::{input::glyphs_helper::GlyphsHelper, prelude::*};

class_names! {
  #[doc = "Class name for the text caret"]
  TEXT_CARET,
}

#[derive(Declare)]
pub struct Caret {}

impl Compose for Caret {
  fn compose(_this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @IgnorePointer {
        @TextClamp {
          rows: Some(1.),
          @ Void {
            class: TEXT_CARET
          }
        }
      }
    }
    .into_widget()
  }
}

pub fn caret_widget(
  caret: impl StateWatcher<Value = CaretState>, text: impl StateWatcher<Value = Text>,
) -> Widget<'static> {
  fn_widget! {
    let tick_of_layout_ready = BuildCtx::get().window()
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));

      @Caret {
        anchor: pipe!(($text.text.clone(), *$caret)).value_chain(|v|{
          v.sample(tick_of_layout_ready).box_it()
        }).map(move |_| {
          if let Some(glyphs) = $text.glyphs() {
            let pos = glyphs.cursor($caret.caret_position());
            Anchor::from_point(pos)
          } else {
            Anchor::default()
          }
        }),
      }
  }
  .into_widget()
}
