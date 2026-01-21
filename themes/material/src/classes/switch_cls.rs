use ribir_core::{animation::easing::CubicBezierEasing, prelude::*};
use ribir_widgets::switch::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    SWITCH_UNCHECKED,
    style_class! {
      background: Palette::of(BuildCtx::get()).surface_variant(),
      border: Border::all(BorderSide {
        color: Palette::of(BuildCtx::get()).outline().into(),
        width: 2.,
      }),
    },
  );

  classes.insert(
    SWITCH_CHECKED,
    style_class! {
      background: Palette::of(BuildCtx::get()).primary(),
      border: Border::all(BorderSide {
        color: Color::TRANSPARENT.into(),
        width: 2.,
      }),
    },
  );

  classes.insert(SWITCH, |w| {
    let mut w = FatObj::new(w);
    w.with_cursor(CursorIcon::Pointer)
      .with_radius(md::RADIUS_16)
      .with_clamp(BoxClamp::fixed_size(Size::new(52., 32.)));
    w.into_widget()
  });

  classes.insert(SWITCH_THUMB, |w| {
    interactive_layers! {
      radius: md::RADIUS_20,
      ripple_radius: 20.,
      center: true,
      @FatObj {
        y: AnchorY::center(),
        @ { w }
      }
    }
    .into_widget()
  });

  const THUMB_TRANS: EasingTransition<CubicBezierEasing> =
    EasingTransition { duration: md::easing::duration::SHORT4, easing: md::easing::STANDARD };

  classes.insert(SWITCH_THUMB_UNCHECKED, |w| {
    fn_widget! {
      let mut thumb = @FatObj {
        margin: EdgeInsets::only_left(4.),
        clamp: BoxClamp::fixed_size(Size::new(16., 16.)),
        radius: md::RADIUS_8,
        background: Palette::of(BuildCtx::get()).outline(),
        @ { w }
      };

      let enter = @Animate {
        state: (thumb.margin(), thumb.clamp(), thumb.radius()),
        transition: THUMB_TRANS,
        from: (md::EDGES_LEFT_24,
          BoxClamp::fixed_size(Size::new(24., 24.)),
          md::RADIUS_12
        ),
      };

      @(thumb) {
        on_mounted: move |_| enter.run(),
      }
    }
    .into_widget()
  });

  classes.insert(SWITCH_THUMB_CHECKED, |w| {
    fn_widget! {
      let mut thumb = @FatObj {
        margin: md::EDGES_LEFT_24,
        clamp: BoxClamp::fixed_size(Size::new(24., 24.)),
        radius: md::RADIUS_12,
        background: Palette::of(BuildCtx::get()).on_primary(),
        @ { w }
      };

      let enter = @Animate {
        state: (thumb.margin(), thumb.clamp(), thumb.radius()),
        transition: THUMB_TRANS,
        from: (EdgeInsets::only_left(4.), BoxClamp::fixed_size(Size::new(16., 16.)), md::RADIUS_8),
      };
      @(thumb) {
        on_mounted: move |_| enter.run(),
      }
    }
    .into_widget()
  });
}
