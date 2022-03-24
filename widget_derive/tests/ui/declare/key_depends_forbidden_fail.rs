use ribir::prelude::*;

fn id_must_be_unique_err(ctx: &mut BuildCtx) {
  declare! {
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
