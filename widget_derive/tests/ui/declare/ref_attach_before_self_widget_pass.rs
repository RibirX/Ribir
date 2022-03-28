use ribir::prelude::*;

#[widget]
fn syntax_pass(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    declare Flex {
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
