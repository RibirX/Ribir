use ribir::prelude::*;

fn main() {
  let size = Size::zero();
  let _child_always_declare_behind_field = widget! {
    SizedBox {
      SizedBox { size }
      size
    }
  };
}
