use crate::layout::{Container, Stack};
use ribir_core::prelude::*;

#[derive(Declare, Declare2)]
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
    fn_widget! {
      let color = SelectedTextStyle::of(ctx!()).brush;
      @Stack {
        @ { pipe!{
          let color = color.clone();
          $this.rects.clone().into_iter().map(move |rc| {
            @Container {
              background: color.clone(),
              top_anchor: rc.origin.y,
              left_anchor: rc.origin.x,
              size: rc.size,
            }
          })
        }}
      }
    }
    .into()
  }
}
