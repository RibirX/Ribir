use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(RADIO, |w| {
    let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(md::RADIUS_20));
    ripple! {
      radius: 20.,
      center: true,
      cursor: CursorIcon::Pointer,
      @ $hover_layer {
        clamp: BoxClamp::fixed_size(md::SIZE_40),
        @ { w }
      }
    }
    .into_widget()
  });

  fn icon_with_ripple<'w>(icon: Widget<'w>, ripple: Widget<'w>, foreground: Color) -> Widget<'w> {
    stack! {
      margin: md::EDGES_4,
      foreground,
      @Icon {
        clamp: BoxClamp::fixed_size(md::SIZE_40),
        text_line_height: 20.,
        @ { icon }
      }
      @{ ripple }
    }
    .into_widget()
  }

  classes.insert(RADIO_SELECTED, |ripple| {
    let color = BuildCtx::get().variant_color();
    let icon = rdl! {
      let  w = @Container {
        size: md::SIZE_10,
        background: color,
        border_radius: md::RADIUS_5,
        h_align: HAlign::Center,
        v_align: VAlign::Center,
      };
      let scale_in = @Animate {
        state: part_writer!(&mut w.size),
        transition: EasingTransition {
          duration: md::easing::duration::SHORT3,
          easing: md::easing::EMPHASIZED_DECELERATE,
        }.box_it(),
        from: ZERO_SIZE,
      };
      @Container {
        size: md::SIZE_20,
        border: md::border_variant_color_2(),
        border_radius: md::RADIUS_10,
        on_mounted: move |_| scale_in.run(),
        @ { w }
      }
    };

    icon_with_ripple(icon.into_widget(), ripple, color)
  });
  classes.insert(RADIO_UNSELECTED, |ripple| {
    let foreground = Palette::of(BuildCtx::get()).on_surface_variant();
    let icon = container! {
      size: md::SIZE_20,
      border: md::border_on_surface_variant_2(),
      border_radius: md::RADIUS_10,
    };
    icon_with_ripple(icon.into_widget(), ripple, foreground)
  });
}
