use ribir::prelude::*;

fn main() {
  let x = 1.;
  let y = 1.;

  let _multi_track = widget! {
    track { x: x.into_stateful() }
    track { y: y.into_stateful() }
    Void {
      left_anchor: x.clone(),
      top_anchor: y.clone(),
    }
  };

  let _track_and_try_track = widget_try_track! {
    try_track { x: x.into() }
    track { y: y.into_stateful() }
    Void {
      left_anchor: x.clone(),
      top_anchor: y.clone(),
    }
  };
}
