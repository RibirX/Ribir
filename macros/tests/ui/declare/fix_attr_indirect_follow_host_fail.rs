use ribir::prelude::*;

fn main() {
  let _fix_builtin_indirect_follow_host_widget_pass = widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
      cursor: b.cursor,
      SizedBox {
        id: b,
        size: Size::zero(),
        cursor: if a.size.area() > 0. {
          CursorIcon::Hand
        } else {
          CursorIcon::Arrow
        } ,
      }
    }
  };

  let _fix_attr_indirect_follow_host_attr_fail = widget! {
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
