use ribir::prelude::*;

#[widget]
fn child_always_declare_behind_field(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    SizedBox {
      SizedBox { size },
      size
    }
  };
}

fn main() {}
