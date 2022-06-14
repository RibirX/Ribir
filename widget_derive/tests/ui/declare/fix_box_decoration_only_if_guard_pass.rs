use ribir::prelude::*;

fn main() {
  widget! {
    SizedBox {
      size: Size::zero(),
      background if true => : Color::RED,
    }
  };
}
