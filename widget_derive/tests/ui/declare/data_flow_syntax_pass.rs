use ribir::prelude::*;

fn data_flow_simple(ctx: &mut BuildCtx) {
  let _ = declare! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        id: b,
        size: Size::zero(),
      }
    }
    data_flow! { a.size ~> b.size }
  };
}

fn data_flow_embed(ctx: &mut BuildCtx) {
  let _ = declare! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        id: b,
        size: Size::zero(),
      }
      true.then(||{
        declare!{
          SizedBox {
            id: c,
            size: Size::zero(),
          }
          data_flow! { a.size + b.size ~> c.size }
        }
      })
    }
    data_flow! { a.size ~> b.size }
  };
}

fn main() {}
