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
    let w = FatObj::new(w)
      .text_line_height(20.)
      .cursor(CursorIcon::Pointer);

    if DisabledRipple::get(BuildCtx::get()) {
      // 24x24 if no ripple
      return w.margin(md::EDGES_2).into_widget();
    }

    let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(md::RADIUS_20));
    ripple! {
      margin: md::EDGES_4,
      ripple_radius: 20.,
      center: true,
      clamp: BoxClamp::fixed_size(md::SIZE_40),
      @ $hover_layer { @ { w } }
    }
    .into_widget()
  });

  classes.insert(RADIO_SELECTED_ICON, |w| {
    let w = FatObj::new(w);
    rdl! {
      let mut bullet = @ $w  {
        clamp: BoxClamp::fixed_size(md::SIZE_10),
        background: BuildCtx::color(),
        radius: md::RADIUS_5,
        h_align: HAlign::Center,
        v_align: VAlign::Center,
      };

      let scale_in = @Animate {
        state: part_writer!(&mut bullet.clamp),
        transition: EasingTransition {
          duration: md::easing::duration::SHORT3,
          easing: md::easing::EMPHASIZED_DECELERATE,
        }.box_it(),
        from: BoxClamp::fixed_size(ZERO_SIZE),
      };
      @Container {
        size: md::SIZE_20,
        border: md::border_2(),
        radius: md::RADIUS_10,
        on_mounted: move |_| scale_in.run(),
        @ { bullet }
      }
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
