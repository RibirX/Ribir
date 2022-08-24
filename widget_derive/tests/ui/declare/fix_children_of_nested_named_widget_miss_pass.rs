use ribir::prelude::*;

fn main() {
  widget! {
    SizedBox {
      size: Size::zero(),
      SizedBox {
        id: id1,
        size: Size::zero(),
        SizedBox { size: Size::zero() }
      }
    }
  };
}
