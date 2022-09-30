use ribir::prelude::*;

fn main() {
  let _flow_simple = widget! {
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
    modify_on a.size ~> b.size
  };

  let _flow_handler = widget! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
        tap: move |_| {}
      }
      SizedBox {
        id: b,
        size: a.size,
      }
      SizedBox {
        id: c,
        size: Size::zero(),
      }
    }

    modify_on a.size + b.size ~> c.size
    on a.size + b.size {
      change : move |(_, after)| c.size = after
    }
    on a { tap: move |_| {} }
    on a.size { change: move |_| {} }
  };

  let _flow_embed = widget! {
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
            modify_on a.size + b.size ~> c.size
          }
        })
      }
    }
    modify_on a.size ~> b.size
  };

  let _fix_named_obj_moved_in_flow = widget! {
    Flex {
      SizedBox { id: a, size: Size::zero() }
      SizedBox { id: b, size: Size::zero() }
      SizedBox { id: c, size: Size::zero() }
    }
    modify_on a.size ~> b.size
    modify_on a.size ~> c.size
  };
}
