use ribir::prelude::*;
fn main() {
  App::run_with_data(
    || Stateful::new(0),
    move |cnt: &'static Stateful<i32>| {
      row! {
        @Button {
          on_tap: move |_| *$write(cnt) += 1,
          @ { "Increment" }
        }
        @ {
          pipe!(*$read(cnt)).map(move |cnt| {
            (0..cnt).map(move |_| {
              @Container {
                margin: EdgeInsets::all(2.),
                size: Size::new(10., 10.),
                background: Color::RED
              }
            })
          })
        }
      }
    },
  );
}
