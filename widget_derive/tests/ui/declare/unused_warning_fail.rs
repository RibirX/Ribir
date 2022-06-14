use ribir::prelude::*;

fn main() {
  compile_error!("Test for declare syntax warning.");
  let _unused_id_warning = widget! {
    SizedBox {
      id: test_id,
      size: Size::zero()
    }
  };
  let _used_id_no_warning = widget! {
    SizedBox {
      id: id1,
      size: Size::new(100., 100.),
      SizedBox {
        size: id1.size,
      }
    }
  };
}
