#![feature(negative_impls)]
#![feature(core_intrinsics)]

use animation::curve::ease_in_expo;
use animation::ticker_animation_mgr::new_ticker_animation_mgr;
use ribir::animation::RepeatMode;

use ribir::prelude::*;
use std::time::Duration;

fn main() {
  // let demo = widget! {
  //   Row {
  //     SizedBox {
  //       id: sized_box,
  //       background: Brush::Color(Color::BLUE),
  //       radius: Radius::all(20.),
  //       size: Size::new(20., 20.),
  //     }

  //     SizedBox {
  //       size: Size::new(220., 20.),
  //       on_tap: move |_| {
  //         let s = sized_box.size;
  //         sized_box.radius =
  // Some(Radius::all(sized_box.radius.unwrap().top_left * 2.));
  //         sized_box.size = Size::new(s.width * 2. , s.height * 2.);
  //       },
  //       Text { text:"click me to trigger animation" }
  //     }
  //   }
  //   animations {
  //     State {
  //       id: state1,
  //       sized_box.size: Size::new(10., 10.),
  //       sized_box.radius: Some(Radius::all(0.)),
  //       sized_box.background: Some(Brush::Color(Color::RED)),
  //     }

  //     Transition {
  //       id: transition1,
  //       duration: Duration::from_secs(5),
  //       repeat: RepeatMode::None,
  //       ease: Some(ease_in_expo as fn() -> Box<dyn Curve>),
  //     }
  //     Animate {
  //       id: animate1,
  //       from: state1,
  //       transition: transition1,
  //     }
  //     sized_box.size: animate1
  //   }
  // };
  // Application::new().run(demo.into_widget(), Some(new_ticker_animation_mgr()));
  todo!();
}
