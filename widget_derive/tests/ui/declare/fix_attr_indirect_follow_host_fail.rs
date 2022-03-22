use ribir::prelude::*;

fn fix_attr_indirect_follow_host_widget_pass(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
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

fn fix_attr_indirect_follow_host_attr_fail(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
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
