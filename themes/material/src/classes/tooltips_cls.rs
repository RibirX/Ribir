use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(TOOLTIPS, |w| {
    fn_widget! {
      let mut w = FatObj::new(w);
      let mut w = @FatObj {
        background: Palette::of(BuildCtx::get()).inverse_surface(),
        margin: EdgeInsets::only_bottom(4.),
        radius: Radius::all(4.),
        @(w) {
          margin: EdgeInsets::new(4., 8., 4., 8.),
          foreground: Palette::of(BuildCtx::get()).inverse_on_surface(),
          x: AnchorX::center(),
          y: AnchorY::center(),
        }
      };
      let opacity = w.opacity();

      @AnimatedPresence {
        cases: cases! {
          state: opacity,
          true => 1.0,
          false => 0.0,
        },
        leave: EasingTransition {
          easing: md::easing::STANDARD_ACCELERATE,
          duration: md::easing::duration::SHORT2,
        },
        @ { w }
      }
    }
    .into_widget()
  });
}
