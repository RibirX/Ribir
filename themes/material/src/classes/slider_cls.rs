use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

class_names! {
  BASE_SLIDER_TRACK,
}

pub(super) fn init(classes: &mut Classes) {
  classes.insert(BASE_SLIDER_TRACK, |w| {
    fn_widget! {
      let flex = Provider::of::<Stateful<Expanded>>(BuildCtx::get()).unwrap();
      part_writer!(&mut flex.flex).transition(
        EasingTransition {
          easing: easing::LinearEasing,
          duration: md::easing::duration::SHORT2,
      });
      let w = FatObj::new(w);
      @ $w {
        clamp: BoxClamp::fixed_height(16.),
      }
    }
    .into_widget()
  });

  classes.insert(SLIDER_ACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        class: BASE_SLIDER_TRACK,
        background: Palette::of(BuildCtx::get()).primary(),
        border_radius: Radius::new(8., 2., 8., 2.),
      }
    }
    .into_widget()
  });

  classes.insert(SLIDER_INACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ $w {
        class: BASE_SLIDER_TRACK,
        background: Palette::of(BuildCtx::get()).secondary_container(),
        border_radius: Radius::new(2., 8., 2., 8.),
      }
    }
    .into_widget()
  });

  classes.insert(SLIDER_INDICATOR, |w| {
    fn_widget! {
      let w = FatObj::new(w);
      @ Cursor {
        cursor: CursorIcon::Pointer,
        @ $w {
          v_align: VAlign::Center,
          background: Palette::of(BuildCtx::get()).primary(),
          margin: EdgeInsets::horizontal(6.),
          clamp: BoxClamp::fixed_size(Size::new(4., 44.)),
        }
      }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_INACTIVE_TRACK_LEFT, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          class: BASE_SLIDER_TRACK,
          border_radius: Radius::new(8., 2., 8., 2.),
          background: Palette::of(BuildCtx::get()).secondary_container(),
        }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_INACTIVE_TRACK_RIGHT, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          class: BASE_SLIDER_TRACK,
          border_radius: Radius::new(2., 8., 2., 8.,),
          background: Palette::of(BuildCtx::get()).secondary_container(),
        }
    }
    .into_widget()
  });

  classes.insert(RANGE_SLIDER_ACTIVE_TRACK, |w| {
    fn_widget! {
      let w = FatObj::new(w);
        @ $w {
          class: BASE_SLIDER_TRACK,
          border_radius: Radius::new(2., 2., 2., 2.,),
          background: Palette::of(BuildCtx::get()).primary(),
        }
    }
    .into_widget()
  });
}
