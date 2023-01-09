use super::InputTheme;
use crate::layout::Container;
use ribir_core::prelude::*;
use std::time::Duration;
#[derive(Declare)]
pub struct Caret {
  #[declare(default = InputTheme::of(ctx).caret_color.clone())]
  pub color: Brush,
  pub size: Size,
}

impl Compose for Caret {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      Container {
        id: caret,
        opacity: 1.,
        background: this.color.clone(),
        mounted: move |_| animate1.run(),
        size: this.size,
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
