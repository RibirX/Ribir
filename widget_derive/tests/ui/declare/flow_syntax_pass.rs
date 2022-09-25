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
    on a.size ~> b.size
  };

  let _flow_handler = widget! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
        on_tap: move |e| {}
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
    on a {
      on_tap: move |_| {},
      change: move |_| {}
    }
    on a.size { change: move |(before, after)| {} }
    on a.size + b.size ~> c.size
    on a.size + b.size {
      change : move |(_, after)| c.size = after
    }
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
            on a.size + b.size ~> c.size
          }
        })
      }
    }
    on a.size ~> b.size
  };

  let _fix_named_obj_moved_in_flow = widget! {
    Flex {
      SizedBox { id: a, size: Size::zero() }
      SizedBox { id: b, size: Size::zero() }
      SizedBox { id: c, size: Size::zero() }
    }
    on a.size ~> b.size
    on a.size ~> c.size
  };
}
