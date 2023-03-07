use ribir::prelude::*;

fn main() {
  widget! {
    SizedBox {
      size: Size::zero(),
      SizedBox {
        id: _id1,
        size: Size::zero(),
        SizedBox { size: Size::zero() }
      }
    }
  };
}
