use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let _ = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero(),
    }
    animations {
      Animate {
        id: size_animate,
        from: State {
          sized_box.size: Size::new(10., 10.),
        },
        transition: Transition {
          duration: Duration::from_secs(5),
          easing: easing::EASE_IN_OUT,
        },
      }
      sized_box.size: size_animate
    }
  };
}
