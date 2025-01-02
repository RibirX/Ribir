use ribir_core::prelude::*;
use ribir_widgets::{input::TEXT_CARET, prelude::*};

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(TEXT_CARET, |_w| {
    fn_widget! {
      let mut w = @ FittedBox {
          box_fit: BoxFit::CoverY,
          @ { svgs::TEXT_CARET }
      };
      let blink_interval = Duration::from_millis(500);
      let unsub = interval(blink_interval, AppCtx::scheduler())
          .subscribe(move |idx| $w.write().opacity = (idx % 2) as f32);
      @ $w {
        on_disposed: move |_| unsub.unsubscribe()
      }
    }
    .into_widget()
  });

  classes.insert(TEXT_HIGH_LIGHT, style_class! {
    background: Color::from_rgb(181, 215, 254),
    border_radius: md::RADIUS_2,
  });
}
