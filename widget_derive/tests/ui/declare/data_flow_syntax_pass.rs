use ribir::prelude::*;

fn data_flow_simple() {
  let _ = declare! {
    Flex {
      ..<_>::default(),
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

fn data_flow_embed() {
  let _ = declare! {
    Flex {
      ..<_>::default(),
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

fn main() {
  data_flow_simple();
  data_flow_embed();
}
