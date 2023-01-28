use super::InputTheme;
use crate::layout::Container;
use ribir_core::prelude::*;
use std::time::Duration;
#[derive(Declare)]
pub struct Caret {
  #[declare(default = InputTheme::of(ctx).caret_color.clone())]
  pub color: Brush,
  pub focused: bool,
  pub size: Size,
}

impl Compose for Caret {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      Container {
        id: caret,
        visible: false,
        background: this.color.clone(),
        size: this.size,
      }
      Animate {
        id: animate1,
        prop: prop!(caret.visible),
        from: true,
        transition: Transition {
          easing: easing::steps(2, easing::StepsJump::JumpNone),
          duration: Duration::from_secs(1),
          repeat: Some(f32::INFINITY),
          delay: None
        }
      }
      finally {
        let_watch!(this.focused)
          .distinct_until_changed()
          .subscribe(move |focused| {
            if focused {
              animate1.run();
            } else {
              animate1.stop();
            }
          });
      }
    }
  }
}
