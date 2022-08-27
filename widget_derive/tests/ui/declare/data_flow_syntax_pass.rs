use ribir::prelude::*;

fn main() {
  let _data_flow_simple = widget! {
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
    dataflows { a.size ~> b.size }
  };

  let _data_flow_embed = widget! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        id: b,
        size: Size::zero(),
      }
      ExprWidget {
        expr: true.then(||{
          widget!{
            SizedBox {
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

  let _fix_named_obj_moved_in_data_flow = widget! {
    Flex {
      SizedBox { id: a, size: Size::zero() }
      SizedBox { id: b, size: Size::zero() }
      SizedBox { id: c, size: Size::zero() }
    }
    dataflows {
      a.size ~> b.size,
      a.size ~> c.size
    }
  };
}
