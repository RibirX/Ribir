use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::InteractiveLayer;

const TOTAL_SIZE: f32 = 40.;
const CONTAINER_SIZE: f32 = 20.;
const INDICATOR_SIZE: f32 = 10.;
const BORDER_SIZE: f32 = 2.;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(RADIO_SELECTED, |_w| {
    fn_widget! {
      let ctx = BuildCtx::get();
      @InteractiveLayer {
        border_radii: Radius::all(TOTAL_SIZE / 2.),
        color: Palette::of(ctx).primary(),
        @Container {
          size: Size::new(TOTAL_SIZE, TOTAL_SIZE),
          cursor: CursorIcon::Pointer,
          @Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: Size::new(CONTAINER_SIZE, CONTAINER_SIZE),
            border_radius: Radius::all(CONTAINER_SIZE / 2.),
            border: Border::all(
              BorderSide::new(BORDER_SIZE, Palette::of(ctx).primary().into())
            ),
            @Container {
              v_align: VAlign::Center,
              h_align: HAlign::Center,
              size: Size::new(INDICATOR_SIZE, INDICATOR_SIZE),
              border_radius: Radius::all(INDICATOR_SIZE / 2.),
              background: Palette::of(ctx).primary(),
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
        border_radii: Radius::all(TOTAL_SIZE / 2.),
        color: Palette::of(ctx).primary(),
        @Container {
          cursor: CursorIcon::Pointer,
          size: Size::new(TOTAL_SIZE, TOTAL_SIZE),
          @Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: Size::new(CONTAINER_SIZE, CONTAINER_SIZE),
            border_radius: Radius::all(CONTAINER_SIZE / 2.),
            border: Border::all(
              BorderSide::new(BORDER_SIZE, Palette::of(ctx).on_surface_variant().into())
            ),
          }
        }
      }
    }
    .into_widget()
  });
}
