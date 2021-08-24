use ribir::prelude::*;

fn child_always_declare_behind_field() {
  let size = Size::zero();
  let _ = declare! {
    SizedBox {
      SizedBox { size },
      size
    }
  };
}

fn main() {}
