use ribir::prelude::*;

#[widget]
fn syntax_pass(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    declare SizedBox {
      id: a,
      size,
      background: Color::RED,
      SizedBox{
        size,
        background: a.background.clone()
      }
    }
  };
}

fn main() {}
