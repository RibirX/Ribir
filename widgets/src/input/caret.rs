use ribir_core::{prelude::*, ticker::FrameMsg};
use std::{cell::RefCell, rc::Rc, time::Duration};

use crate::layout::Container;

use super::{input_text::GlyphsHelper, CaretState};
#[derive(Declare)]
pub struct CaretStyle {
  pub font: TextStyle,
}

impl ComposeStyle for CaretStyle {
  type Host = Widget;
  fn compose_style(this: Stateful<Self>, host: Self::Host) -> Widget
  where
    Self: Sized,
  {
    widget! {
      states { this }
      DynWidget {
        id: caret,
        opacity: 1.,
        background: this.font.foreground.clone(),
        mounted: move |_| animate1.run(),
        dyns: host,
      }
      Animate {
        id: animate1,
        prop: prop!(caret.opacity),
        from: 0.,
        transition: Transition {
          easing: easing::steps(2, easing::StepsJump::JumpNone),
          duration: Duration::from_secs(1),
          repeat: Some(f32::INFINITY),
          delay: None
        }
      }
    }
  }
}

#[derive(Declare)]
pub(crate) struct Caret {
  pub(crate) caret: CaretState,
  pub(crate) font: TextStyle,
  pub(crate) glyphs_helper: Rc<RefCell<GlyphsHelper>>,
}

impl Compose for Caret {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states {this: this.into_stateful()}
      CaretStyle{
        id: caret,
        font: this.font.clone(),
        top_anchor: 0.,
        left_anchor: 0.,
        Container {
          id: icon,
          size: Size::new(1., 0.),
        }
      }

      finally {
        let_watch!(this.caret)
          .distinct_until_changed()
          .sample(ctx.app_ctx().frame_tick_stream().filter(|msg| matches!(msg, FrameMsg::LayoutReady(_))))
          .subscribe(move |_| {
            let (offset, height) = this.glyphs_helper.borrow().cursor(this.caret.offset());
            caret.top_anchor = PositionUnit::Pixel(offset.y);
            caret.left_anchor = PositionUnit::Pixel(offset.x);
            icon.size = Size::new(1., height);
          });

        // emit the first elem to caret stream after mounted
        this.caret = this.caret;
      }
    }
  }
}
