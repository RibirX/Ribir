use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let w = widget! {
    Row {
      SizedBox {
        id: sized_box,
        background: Brush::Color(Color::BLUE),
        border_radius: Radius::all(20.),
        size: Size::new(20., 20.),
      }
      Text {
        text:"click me to trigger animation",
        tap: move |_| {
          let s = sized_box.size;
          sized_box.border_radius = Some(Radius::all(sized_box.border_radius.unwrap().top_left * 2.));
          sized_box.size = Size::new(s.width * 2. , s.height * 2.);
        }
      }
    }
    transition (
      prop!(sized_box.size),
      prop!(sized_box.border_radius),
      prop!(sized_box.background)
    ) {
      easing: easing::EASE_IN_OUT,
      duration: Duration::from_secs(5)
    }
  };

  app::run(w);
}
