use ribir_core::prelude::*;

use crate::layout::{Container, Stack};

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
  pub(crate) rects: Vec<Rect>,
}

impl Compose for SelectedText {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      Stack {
        DynWidget {
          dyns: {
            this.rects.iter().map(move |rc: &Rect| rc.clone())
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
    }
  }
}
