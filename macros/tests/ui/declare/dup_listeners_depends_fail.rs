use ribir::prelude::*;

fn main() {
  let _double_tap_instance_have = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero(),
      tap: |_| {}
    }
    on sized_box {
      tap: |_| {}
    }
    on sized_box.tap {
      modify: |_| {}
    }
  };
}
