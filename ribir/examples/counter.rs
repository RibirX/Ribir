use ribir::prelude::*;

fn main() {
  let w = widget! {
    states {
    cnt: Stateful::new(0_i32),
    }
    Row {
      margin: EdgeInsets::all(2.),
      Button {
        on_tap: move |_| *cnt += 1,
        margin: EdgeInsets::only_right(2.),
        ButtonText::new("Add")
      }
      Button {
        on_tap: move |_| *cnt -= 1,
        margin: EdgeInsets::only_right(2.),
        ButtonText::new("Sub")
      }
      Text {
        text: {
          let cnt = *cnt;
          format!("current count: {cnt}")
        }
      }
    }
  };
  app::run(w);
}
