use ribir::prelude::*;

#[widget]
fn id_must_be_unique_err(_this: (), ctx: &mut BuildCtx) {
  widget! {
    SizedBox {
      id: key1,
      key: "key1",
      size: Size::zero(),
      SizedBox {
        key:  key1.key,
        size: Size::zero(),
      }
    }
  };
}

fn main() {}
