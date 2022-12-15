use std::{cell::RefCell, rc::Rc};

use ribir_core::{prelude::*, ticker::FrameMsg};

use crate::layout::{Container, Stack};

use super::{input_text::GlyphsHelper, CaretState};

#[derive(Declare)]
pub struct SelectedTextStyle {}

impl ComposeStyle for SelectedTextStyle {
  type Host = Widget;
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget
  where
    Self: Sized,
  {
    widget! {
      DynWidget {
        background: Color::from_rgb(181, 215, 254), // todo: follow application active state
        dyns: host,
      }
    }
  }
}

#[derive(Declare)]
pub(crate) struct SelectedText {
  pub(crate) caret: CaretState,
  pub(crate) glyphs_helper: Rc<RefCell<GlyphsHelper>>,
}

impl Compose for SelectedText {
  fn compose(this: StateWidget<Self>) -> Widget {
    let rects = vec![];
    
    widget! {
      states {
        this: this.into_stateful(),
        rects: rects.into_stateful(),
      }
      init ctx => {
        let tick_of_layout_ready = ctx.app_ctx().frame_tick_stream().filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
      }
      Stack {
        DynWidget {
          dyns: {
            rects.iter().map(move |rc: &Rect| rc.clone())
            .map(|rc| {
            widget! {
              SelectedTextStyle {
                top_anchor: rc.origin.y,
                left_anchor: rc.origin.x,
                Container {
                  size: rc.size.clone(),
                }
              }
            }
          }).collect::<Vec<_>>()}
        }
      }

      finally {
        let_watch!(this.caret.clone())
          .distinct_until_changed()
          .sample(tick_of_layout_ready)
          .subscribe(move |caret| {
            *rects =   this.glyphs_helper.borrow().selection(caret.select_range());
          });
        }
    }
  }
}
