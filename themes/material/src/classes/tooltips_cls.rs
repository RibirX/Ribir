use ribir_core::prelude::*;

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(TOOLTIPS, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      let mut w = @BoxDecoration {
        background: Palette::of(BuildCtx::get()).inverse_surface(),
        margin: EdgeInsets::only_bottom(4.),
        border_radius: Radius::all(4.),
        @ $w {
          margin: EdgeInsets::new(4., 8., 4., 8.),
          foreground: Palette::of(BuildCtx::get()).inverse_on_surface(),
          v_align: VAlign::Center,
          h_align: HAlign::Center,
        }
      };
      let animate = part_writer!(&mut w.opacity)
        .transition(EasingTransition{
          easing: md::easing::STANDARD_ACCELERATE,
          duration: md::easing::duration::SHORT2
        }.box_it());
      @ $w {
        keep_alive: pipe!($animate.is_running() || $w.opacity != 0.),
        on_disposed: move |_| {
          $w.write().opacity = 0.;
        }
      }
    }
    .into_widget()
  });
}
