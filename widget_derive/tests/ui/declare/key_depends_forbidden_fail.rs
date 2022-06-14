use ribir::prelude::*;

fn main() {
  let _id_must_be_unique_err = widget! {
    SizedBox {
      id: key1,
      key: "key1",
      size: Size::zero(),
      SizedBox {
        key:  key1.key.clone(),
        size: Size::zero(),
      }
    }
  };
}
