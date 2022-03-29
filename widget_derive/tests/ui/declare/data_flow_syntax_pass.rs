use ribir::prelude::*;

#[widget]
fn data_flow_simple(_this: (), ctx: &mut BuildCtx) {
  let _ = widget! {
    declare Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        id: b,
        size: Size::zero(),
      }
    }
    dataflows { a.size ~> b.size }
  };
}

#[widget]
fn data_flow_embed(_this: (), ctx: &mut BuildCtx) {
  let _ = widget! {
    declare Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        id: b,
        size: Size::zero(),
      }
      ExprChild {
        true.then(||{
          widget!{
            declare SizedBox {
              id: c,
              size: Size::zero(),
            }
            dataflows { a.size + b.size ~> c.size }
          }
        })
      }
    }
    dataflows { a.size ~> b.size }
  };
}

fn main() {}
