use super::InputStyle;
use crate::{layout::SizedBox, themes::svgs};
use ribir_core::prelude::*;
use std::time::Duration;
#[derive(Declare)]
pub struct Caret {
  #[declare(default = InputStyle::of(ctx).caret_color.clone())]
  pub color: Brush,
  pub focused: bool,
  pub height: f32,
  #[declare(default = svgs::TEXT_CARET)]
  pub icon: NamedSvg,
}

impl Compose for Caret {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_readonly() }
      SizedBox {
        left_anchor: -this.height / 2.,
        size: Size::new(this.height, this.height),
        DynWidget {
          id: caret,
          visible: false,
          dyns: this.icon,
          box_fit: BoxFit::Fill,
        }
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
    .into_widget()
  }
}
