  use ribir::prelude::*;

fn main() {
  compile_error!("Test for declare syntax warning.");
}

#[widget]
fn unused_id_warning(_this: (), ctx: &mut BuildCtx) {
  widget! {
    declare SizedBox {
      id: test_id,
      size: Size::zero()
    }
  };
}

#[widget]
fn used_id_no_warning(_this: (), ctx: &mut BuildCtx) {
  widget! {
    declare SizedBox {
      id: id1,
      size: Size::new(100., 100.),
      SizedBox {
        size: id1.size,
      }
    }
  };
}
