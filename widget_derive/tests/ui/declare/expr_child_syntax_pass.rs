use ribir::prelude::*;

#[widget]
fn child_always_declare_behind_field(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    declare SizedBox {
      size,
      ExprChild {
        if size.area() > 0. {
          SizedBox { size }
        } else {
          SizedBox { size }
        }
      }
    }
  };
}

#[widget]
fn option_child(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    declare SizedBox {
      size,
      background: Color::RED,
      ExprChild {
        (size.area() == 0.).then(||{
          SizedBox { size }
        })
      }
    }
  };
}

#[widget]
fn expr_child_use_named_widget(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    declare Flex {
      SizedBox {
        id: a,
        size,
      }
      ExprChild {
        (a.size.area() > 0.).then(||
          SizedBox {
            size,
        })
      }
    }
  };
}

fn main() {}
