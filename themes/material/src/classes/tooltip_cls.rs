use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

const TOOLTIP_ENTER_EASING: easing::CubicBezierEasing =
  easing::CubicBezierEasing::new(0., 0., 0.2, 1.);
const TOOLTIP_EXIT_EASING: easing::CubicBezierEasing =
  easing::CubicBezierEasing::new(0.4, 0., 1., 1.);
const TOOLTIP_MOTION_DURATION: Duration = Duration::from_millis(150);

fn tooltip_shell_class(w: Widget) -> Widget {
  fn_widget! {
    let mut w = FatObj::new(w);
    let opacity = w.opacity();

    @AnimatedPresence {
      cases: cases! {
        state: opacity,
        true => 1.0,
        false => 0.0,
      },
      enter: EasingTransition {
        easing: TOOLTIP_ENTER_EASING,
        duration: TOOLTIP_MOTION_DURATION,
      },
      leave: EasingTransition {
        easing: TOOLTIP_EXIT_EASING,
        duration: TOOLTIP_MOTION_DURATION,
      },
      interruption: Interruption::Fluid,
      @ { w }
    }
  }
  .into_widget()
}

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    TOOLTIP,
    style_class! {
      margin: EdgeInsets::only_bottom(4.),
      padding: EdgeInsets::symmetrical(4., 8.),
      radius: Radius::all(4.),
      background: Palette::of(BuildCtx::get()).inverse_surface(),
      foreground: Palette::of(BuildCtx::get()).inverse_on_surface(),
    },
  );
  classes.insert(TOOLTIP_SHELL, tooltip_shell_class);
}
