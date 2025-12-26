use ribir_core::{animation::easing::CubicBezierEasing, prelude::*};
use ribir_widgets::switch::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    SWITCH_UNCHECKED,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::new(52., 32.)),
      background: Palette::of(BuildCtx::get()).surface_variant(),
      border: Border::all(BorderSide {
        color: Palette::of(BuildCtx::get()).outline().into(),
        width: 2.,
      }),
      radius: md::RADIUS_16,
    },
  );

  classes.insert(
    SWITCH_CHECKED,
    style_class! {
      clamp: BoxClamp::fixed_size(Size::new(52., 32.)),
      background: Palette::of(BuildCtx::get()).primary(),
      radius: md::RADIUS_16,
    },
  );

  const THUMB_TRANS: EasingTransition<CubicBezierEasing> =
    EasingTransition { duration: md::easing::duration::SHORT4, easing: md::easing::STANDARD };

  classes.insert(SWITCH_THUMB_UNCHECKED, |w| {
    fn_widget! {
      let mut container = @FatObj {
        v_align: VAlign::Center,
        clamp: BoxClamp::fixed_size(Size::new(16., 16.)),
        radius: md::RADIUS_8,
        background: Palette::of(BuildCtx::get()).outline(),
        margin: EdgeInsets::only_left(8.)
        @ { w }
      };

      let enter = @Animate {
        state: (container.margin(), container.clamp(), container.radius()),
        transition: THUMB_TRANS,
        from: (EdgeInsets::only_left(24.),
          BoxClamp::fixed_size(Size::new(24., 24.)),
          md::RADIUS_12
        ),
      };

      @(container) {
        on_mounted: move |_| enter.run(),
      }
    }
    .into_widget()
  });

  classes.insert(SWITCH_THUMB_CHECKED, |w| {
    fn_widget! {
      let mut container = @FatObj {
        clamp: BoxClamp::fixed_size(Size::new(24., 24.)),
        v_align: VAlign::Center,
        radius: md::RADIUS_12,
        background: Palette::of(BuildCtx::get()).on_primary(),
        margin: EdgeInsets::only_left(24.),
        @ { w }
      };

      let enter = @Animate {
        state: (container.margin(), container.clamp(), container.radius()),
        transition: THUMB_TRANS,
        from: (EdgeInsets::only_left(8.), BoxClamp::fixed_size(Size::new(16., 16.)), md::RADIUS_8),
      };
      @(container) {
        on_mounted: move |_| enter.run(),
      }
    }
    .into_widget()
  });
}
