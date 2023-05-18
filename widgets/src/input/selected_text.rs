use crate::layout::{Container, Stack};
use ribir_core::prelude::*;

#[derive(Declare)]
pub(crate) struct SelectedText {
  pub(crate) rects: Vec<Rect>,
}

#[derive(Clone, PartialEq)]
pub struct SelectedTextStyle {
  pub brush: Brush,
}
impl CustomStyle for SelectedTextStyle {
  fn default_style(_: &BuildCtx) -> Self {
    SelectedTextStyle {
      brush: Color::from_rgb(181, 215, 254).into(),
    }
  }
}

impl Compose for SelectedText {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      init ctx => {
        let color = SelectedTextStyle::of(ctx).brush.clone();
      }
      Stack {
        DynWidget {
          dyns: {
            this.rects.iter().copied()
            .map(|rc| {
              let color = color.clone();
              widget! {
                  Container {
                    background: color,
                    top_anchor: rc.origin.y,
                    left_anchor: rc.origin.x,
                    size: rc.size,
                  }
              }
            }).collect::<Vec<_>>()
          }
        }
      }
    }
  }
}
