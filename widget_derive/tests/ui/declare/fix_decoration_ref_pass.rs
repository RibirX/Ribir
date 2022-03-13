use ribir::prelude::*;

fn syntax_pass(ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = declare! {
      SizedBox {
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
