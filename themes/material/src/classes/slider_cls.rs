use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

const THUMB_HEIGHT: f32 = 44.;
const TRACK_HEIGHT: f32 = 16.;

const RADIUS_L2_R8: Radius = Radius::new(2., 8., 2., 8.);
const RADIUS_L8_R2: Radius = Radius::new(8., 2., 8., 2.);

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    SLIDER_CONTAINER,
    style_class! {
      cursor: CursorIcon::Pointer,
      clamp: BoxClamp::fixed_height(THUMB_HEIGHT)
    },
  );

  classes.insert(SLIDER_THUMB_CONTAINER, style_class! { margin: md::EDGES_HOR_4 });

  classes.insert(
    SLIDER_ACTIVE_TRACK,
    style_class! {
      background: BuildCtx::color(),
      radius: RADIUS_L8_R2,
      margin: md::EDGES_RIGHT_8,
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    SLIDER_INACTIVE_TRACK,
    style_class! {
      background: BuildCtx::container_color(),
      radius: RADIUS_L2_R8,
      margin: md::EDGES_LEFT_8,
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    SLIDER_THUMB,
    style_class! {
      y: AnchorY::center(),
      background: BuildCtx::color(),
      radius: md::RADIUS_2,
      clamp: BoxClamp::fixed_size(Size::new(md::THICKNESS_4, THUMB_HEIGHT)),
    },
  );

  classes.insert(
    RANGE_SLIDER_INACTIVE_TRACK_LEFT,
    style_class! {
      radius: RADIUS_L8_R2,
      margin: md::EDGES_RIGHT_8,
      background: BuildCtx::container_color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    RANGE_SLIDER_INACTIVE_TRACK_RIGHT,
    style_class! {
      radius: RADIUS_L2_R8,
      margin: md::EDGES_LEFT_8,
      background: BuildCtx::container_color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    RANGE_SLIDER_ACTIVE_TRACK,
    style_class! {
      radius: md::RADIUS_2,
      margin: md::EDGES_HOR_8,
      background: BuildCtx::color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  named_style_impl! { base_tick => {
    radius: md::RADIUS_2,
    clamp: BoxClamp::fixed_size(md::SIZE_4),
  } }

  classes.insert(
    SLIDER_TICK_ACTIVE,
    class_chain_impl![
      style_class! {
        background: BuildCtx::color().on_this_color(BuildCtx::get())
      },
      base_tick
    ],
  );

  classes.insert(
    SLIDER_TICK_INACTIVE,
    class_chain_impl![
      style_class! {
        background: BuildCtx::color().on_this_container_color(BuildCtx::get())
      },
      base_tick
    ],
  );
}
