use ribir::prelude::*;

fn main() {
  let x = 1.;
  let y = 1.;

  let _multi_track = widget! {
    states { x: x.into_stateful() }
    states { y: y.into_stateful() }
    Void {
      left_anchor: x.clone(),
      top_anchor: y.clone(),
    }
  };
}
