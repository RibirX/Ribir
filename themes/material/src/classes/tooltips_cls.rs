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
      let animate = w.opacity()
        .transition(EasingTransition{
          easing: md::easing::STANDARD_ACCELERATE,
          duration: md::easing::duration::SHORT2
        });
      @(w) {
        keep_alive: pipe!($read(animate).is_running() || *$read(w.opacity()) != 0.),
        on_disposed: move |_| {
          *$write(w.opacity()) = 0.;
        }
      }
    }
    .into_widget()
  });
}
