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
        id: first,
        size: Size::zero(),
        tap: move |e| {}
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
    on a.size + b.size: move |_, after| c.size = after



    #3.3
    on a {
      tap: move |_| animate1.run(),
      press: move |e| menu.open(),
      change: move |before, after| { }
    }
    on a.size { change: move |before, after| {} },
    a.size ~> b.size

    animations {
      on a {
        tap: Animate {}
      }

      on a.size {
        change: ctx.theme().xxx_transition
      }
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
