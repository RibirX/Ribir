use crate::layout::SizedBox;
use ribir_core::prelude::*;
use std::time::Duration;
#[derive(Declare)]
pub struct Caret {
  pub focused: bool,
  pub height: f32,
  #[declare(default = svgs::TEXT_CARET)]
  pub icon: NamedSvg,
}

impl Compose for Caret {
  fn compose(this: State<Self>) -> Widget {
    let blink_interval = Duration::from_millis(500);
    widget! {
      states { this: this.into_readonly() }
      SizedBox {
        left_anchor: -this.height / 2.,
        size: Size::new(this.height, this.height),
        DynWidget {
          id: caret,
          opacity: 0.,
          dyns: this.icon,
          box_fit: BoxFit::Fill,
        }
      }
      finally ctx => {
        let scheduler = ctx.wnd_ctx().frame_scheduler();
        let mut _guard = None;
        let_watch!(this.focused)
          .distinct_until_changed()
          .subscribe(move |focused| {
            if focused {
              caret.opacity = 1.;
              let unsub = interval(blink_interval, scheduler.clone())
                            .subscribe(move |_| caret.opacity = 1. - caret.opacity);
              _guard = Some(BoxSubscription::new(unsub).unsubscribe_when_dropped());
            } else {
              caret.opacity = 0.;
              _guard = None;
            }
          });
      }
    }
  }
}
