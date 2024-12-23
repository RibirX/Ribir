use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(RADIO_SELECTED, |_w| {
    fn_widget! {
      let primary = Palette::of(BuildCtx::get()).primary();
      @InteractiveLayer {
        border_radii: md::RADIUS_20,
        color: primary,
        @Container {
          size: md::SIZE_40,
          cursor: CursorIcon::Pointer,
          @Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: md::SIZE_20,
            border_radius: md::RADIUS_10,
            border: md::border_primary_2(),
            @Container {
              v_align: VAlign::Center,
              h_align: HAlign::Center,
              size: md::SIZE_10,
              border_radius: md::RADIUS_5,
              background: primary,
            }
          }
        }
      }
    }
    .into_widget()
  });

  classes.insert(RADIO_UNSELECTED, |_w| {
    fn_widget! {
      let ctx = BuildCtx::get();
      @InteractiveLayer {
        border_radii: md::RADIUS_20,
        color: Palette::of(ctx).primary(),
        @Container {
          cursor: CursorIcon::Pointer,
          size: md::SIZE_40,
          @Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: md::SIZE_20,
            border_radius: md::RADIUS_10,
            border: md::border_on_surface_variant_2(),
          }
        }
      }
    }
    .into_widget()
  });
}
