use ribir::prelude::*;

#[widget]
fn child_always_declare_behind_field(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    SizedBox {
      size,
      if size.area() > 0. {
        SizedBox { size }.box_it()
      } else {
        widget!{
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
    SizedBox {
      size,
      background: Color::RED,
      (size.area() == 0.).then(||{
        SizedBox { size }
      })
    }
  };
}

#[widget]
fn expr_child_use_named_widget(_this: (), ctx: &mut BuildCtx) {
  let size = Size::zero();
  let _ = widget! {
    Flex {
      SizedBox {
        id: a,
        size,
      }
      (a.size.area() > 0.).then(||
        SizedBox {
          size,
        }
      )
    }
  };
}

fn main() {}
