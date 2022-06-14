use ribir::prelude::*;

fn main() {
  let _id_must_be_unique_err = widget! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      SizedBox {
        id: same_id,
        size: Size::zero(),
      }
    }
  };
}
