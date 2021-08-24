use ribir::prelude::*;

fn syntax_pass() {
  let size = Size::zero();
  let _ = declare! {
    Flex {
      ..<_>::default(),
      SizedBox {
        size,
        SizedBox{
          id: grandson,
          size
        }
      }
      SizedBox{
        size: grandson.size
      }
    }
  };
}

fn main() {}
