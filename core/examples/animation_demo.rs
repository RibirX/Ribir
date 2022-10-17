use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let w = widget! {
    Row {
      SizedBox {
        id: sized_box,
        background: Brush::Color(Color::BLUE),
        radius: Radius::all(20.),
        size: Size::new(20., 20.),
      }
      Text {
        text:"click me to trigger animation",
        tap: move |_| {
          let s = sized_box.size;
          sized_box.radius = Some(Radius::all(sized_box.radius.unwrap().top_left * 2.));
          sized_box.size = Size::new(s.width * 2. , s.height * 2.);
        }
      }
    }
    change_on sized_box.size Animate {
      from: State {
        sized_box.size: Size::new(10., 10.),
        sized_box.radius: Some(Radius::all(0.)),
        sized_box.background: Some(Brush::Color(Color::RED)),
      },
      transition: Transition {
        duration: Duration::from_secs(5),
        easing: easing::EASE_IN_OUT,
      }
    }
  };

  Application::new().run(w);
}
