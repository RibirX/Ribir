use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

const INDICATOR_HEIGHT: f32 = 44.;
const TRACK_HEIGHT: f32 = 16.;
const TRACK_WIDTH: f32 = 4.;

const SMALL_RADIUS: f32 = 2.;
const LARGE_RADIUS: f32 = 8.;
const STOP_INDICATOR_MARGIN: EdgeInsets = EdgeInsets::horizontal(6.);
const STOP_INDICATOR_SIZE: Size = Size::new(4., 4.);

macro_rules! stop_indicator_class {
  ($($field: ident: $value: expr),* ) => {
    style_class! {
      v_align: VAlign::Center,
      border_radius: Radius::all(SMALL_RADIUS),
      margin: STOP_INDICATOR_MARGIN,
      clamp: BoxClamp::fixed_size(STOP_INDICATOR_SIZE),
      $($field: $value),*
    }
  };
}

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    SLIDER_CONTAINER,
    style_class!(
      cursor: CursorIcon::Pointer,
      clamp: BoxClamp::fixed_height(INDICATOR_HEIGHT)
    ),
  );
  classes.insert(SLIDER_ACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        background: Palette::of(BuildCtx::get()).primary(),
        border_radius: Radius::new(LARGE_RADIUS, SMALL_RADIUS, LARGE_RADIUS, SMALL_RADIUS),
        clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
      }
    }
    .into_widget()
  });

  classes.insert(SLIDER_INACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        border_radius: Radius::new(SMALL_RADIUS, LARGE_RADIUS, SMALL_RADIUS, LARGE_RADIUS),
        background: Palette::of(BuildCtx::get()).secondary_container(),
        clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
      }
    }
    .into_widget()
  });

  classes.insert(SLIDER_INDICATOR, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        v_align: VAlign::Center,
        background: Palette::of(BuildCtx::get()).primary(),
        border_radius: Radius::all(SMALL_RADIUS),
        margin: EdgeInsets::horizontal(6.),
        clamp: BoxClamp::fixed_size(Size::new(TRACK_WIDTH, INDICATOR_HEIGHT)),
      }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_INACTIVE_TRACK_LEFT, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          border_radius: Radius::new(LARGE_RADIUS, SMALL_RADIUS, LARGE_RADIUS, SMALL_RADIUS),
          background: Palette::of(BuildCtx::get()).secondary_container(),
          clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
        }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_INACTIVE_TRACK_RIGHT, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          border_radius: Radius::new(SMALL_RADIUS, LARGE_RADIUS, SMALL_RADIUS, LARGE_RADIUS,),
          background: Palette::of(BuildCtx::get()).secondary_container(),
          clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
        }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_ACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          border_radius: Radius::all(SMALL_RADIUS),
          background: Palette::of(BuildCtx::get()).primary(),
          clamp: BoxClamp::fixed_height(TRACK_HEIGHT),
        }
    }
    .into_widget()
  });

  classes.insert(STOP_INDICATOR_ACTIVE, stop_indicator_class! {
    background: Palette::of(BuildCtx::get()).on_primary()
  });

  classes.insert(STOP_INDICATOR_INACTIVE, stop_indicator_class! {
    background: Palette::of(BuildCtx::get()).on_secondary_container()
  });
}
