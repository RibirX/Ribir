use ribir::prelude::*;

fn main() {
  let x = 1.;
  let y = 1.;

  let _multi_track = widget! {
    states { x: Stateful::new(x) }
    states { y: Stateful::new(y) }
    Void {
      left_anchor: x.clone(),
      top_anchor: y.clone(),
    }
  };
}
