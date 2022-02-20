use ribir::prelude::*;

fn syntax_pass() {
  let size = Size::zero();
  let _ = declare! {
    Flex {
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
