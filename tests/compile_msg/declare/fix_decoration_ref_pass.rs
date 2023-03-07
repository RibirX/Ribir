use ribir::prelude::*;

fn main() {
  let size = Size::zero();
  let _ = widget! {
    SizedBox {
      id: a,
      size,
      background: Color::RED,
      SizedBox{
        size,
        background: a.background.clone()
      }
    }
  };
}
