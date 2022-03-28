use ribir::prelude::*;

#[widget]
fn fix_attr_indirect_follow_host_widget_pass(_this: (), ctx: &mut BuildCtx) {
  widget! {
    declare SizedBox {
      id: a,
      size: Size::zero(),
      cursor: b.cursor,
      SizedBox {
        id: b,
        size: Size::zero(),
        cursor: if a.size.area() > 0 {
          CursorIcon::Hand
        } else {
          CursorIcon::RightArrow
        } ,
      }
    }
  };
}

#[widget]
fn fix_attr_indirect_follow_host_attr_fail(_this: (), ctx: &mut BuildCtx) {
  widget! {
    declare SizedBox {
      id: a,
      size: Size::zero(),
      cursor: b.cursor,
      SizedBox {
        id: b,
        size: Size::zero(),
        cursor: a.cursor
      }
    }
  };
}

fn main() {}
