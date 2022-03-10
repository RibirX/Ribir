use ribir::prelude::*;

fn main() {
  compile_error!("Test for declare syntax warning.");
}

fn unused_id_warning(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
      id: test_id,
      size: Size::zero()
    }
  };
}

fn used_id_no_warning(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
      id: id1,
      size: Size::new(100., 100.),
      SizedBox {
        size: id1.size,
      }
    }
  };
}
