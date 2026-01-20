use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(RADIO_SELECTED, style_class! { foreground: BuildCtx::color() });

  classes.insert(
    RADIO_UNSELECTED,
    style_class! { foreground: Palette::of(BuildCtx::get()).on_surface_variant()},
  );

  classes.insert(RADIO, |w| {
    let margin = if Provider::of::<DisableInteractiveLayer>(BuildCtx::get()).is_some() {
      EdgeInsets::all(3.)
    } else {
      md::EDGES_4
    };

    interactive_layers! {
      center: true,
      ripple_radius: 20.,
      ring_outer_offset: 2.,

      radius: md::RADIUS_20,
      text_line_height: 20.,
      cursor: CursorIcon::Pointer,
      clamp: BoxClamp::fixed_size(md::SIZE_40),
      margin,
      @ { w }
    }
    .into_widget()
  });

  classes.insert(RADIO_SELECTED_ICON, |w| {
    let mut w = FatObj::new(w);
    rdl! {
      let mut bullet = @(w)  {
        clamp: BoxClamp::fixed_size(md::SIZE_10),
        background: BuildCtx::color(),
        radius: md::RADIUS_5,
        h_align: HAlign::Center,
        v_align: VAlign::Center,
      };

      let scale_in = @Animate {
        state: bullet.clamp(),
        transition: EasingTransition {
          duration: md::easing::duration::SHORT3,
          easing: md::easing::EMPHASIZED_DECELERATE,
        },
        from: BoxClamp::fixed_size(ZERO_SIZE),
      };
      @Container {
        size: md::SIZE_20,
        border: md::border_2(),
        radius: md::RADIUS_10,
        on_mounted: move |_| scale_in.run(),
        @ { bullet }
      }.into_widget()
    }
  });
  classes.insert(
    RADIO_UNSELECTED_ICON,
    style_class! {
      clamp: BoxClamp::fixed_size(md::SIZE_20),
      border: md::border_2_surface_color(),
      radius: md::RADIUS_10,
    },
  );
}
