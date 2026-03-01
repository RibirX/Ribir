use ribir_core::prelude::*;

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

      AnimatePresence {
        enter: None,
        leave: Some(LeaveAction {
          state: opacity,
          transition: EasingTransition {
            easing: md::easing::STANDARD_ACCELERATE,
            duration: md::easing::duration::SHORT2,
          },
          to: 0.,
        }.into()),
      }.with_child(w)
    }
    .into_widget()
  });
}
