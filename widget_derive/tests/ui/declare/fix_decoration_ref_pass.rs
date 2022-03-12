use ribir::prelude::*;

fn syntax_pass(ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = declare! {
      SizedBox {
        id: a,
        size,
        SizedBox{
          size: grandson.size,
          background: a.background
        }
      }
  };
}

fn main() {}
