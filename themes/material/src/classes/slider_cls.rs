use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

const INDICATOR_HEIGHT: f32 = 44.;
const TRACK_HEIGHT: f32 = 16.;

const RADIUS_L2_R8: Radius = Radius::new(2., 8., 2., 8.);
const RADIUS_L8_R2: Radius = Radius::new(8., 2., 8., 2.);

macro_rules! stop_indicator_class {
  ($($field: ident: $value: expr),* ) => {
    style_class! {
      v_align: VAlign::Center,
      radius: md::RADIUS_2,
      margin: md::EDGES_HOR_6,
      clamp: BoxClamp::fixed_size(md::SIZE_4),
      $($field: $value),*
    }
  };
}

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    SLIDER_CONTAINER,
    style_class! {
      cursor: CursorIcon::Pointer,
      clamp: BoxClamp::fixed_height(INDICATOR_HEIGHT)
    },
  );
  classes.insert(
    SLIDER_ACTIVE_TRACK,
    style_class! {
      background: BuildCtx::color(),
      radius: RADIUS_L8_R2,
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    SLIDER_INACTIVE_TRACK,
    style_class! {
      radius: RADIUS_L2_R8,
      background: BuildCtx::container_color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    SLIDER_INDICATOR,
    style_class! {
      v_align: VAlign::Center,
      background: BuildCtx::color(),
      radius: md::RADIUS_2,
      margin: EdgeInsets::horizontal(6.),
      clamp: BoxClamp::fixed_size(Size::new(md::THICKNESS_4, INDICATOR_HEIGHT)),
    },
  );

  classes.insert(
    RANGE_SLIDER_INACTIVE_TRACK_LEFT,
    style_class! {
      radius: RADIUS_L8_R2,
      background: BuildCtx::container_color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    RANGE_SLIDER_INACTIVE_TRACK_RIGHT,
    style_class! {
      radius: RADIUS_L2_R8,
      background: BuildCtx::container_color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    RANGE_SLIDER_ACTIVE_TRACK,
    style_class! {
      radius: md::RADIUS_2,
      background: BuildCtx::color(),
      clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
    },
  );

  classes.insert(
    STOP_INDICATOR_ACTIVE,
    stop_indicator_class! {
      background: BuildCtx::color().on_this_color(BuildCtx::get())
    },
  );

  classes.insert(
    STOP_INDICATOR_INACTIVE,
    stop_indicator_class! {
      background: BuildCtx::color().on_this_container_color(BuildCtx::get())
    },
  );
}
