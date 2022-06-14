use ribir::prelude::*;

fn main() {
  let _if_guard_require_declare_default = widget! {
    SizedBox {
      size if false => : Size::new(100., 100.)
    }
  };

  let guard = Some(1);
  let _normal_if_guard_pass = widget! {
    Checkbox {
      // if guard for widget's field
      checked if true => : true,
      // if guard for built in fields
      cursor if true => : CursorIcon::Hand,
      margin if true => : EdgeInsets::all(1.)
    }
  };

  let _id_if_guard_fail = widget! {
    Checkbox {
      id if true => : test,
      // if guard in widget's field
      check if true => : true,

    }
  };

  let _depend_id_behind_if_guard_fail = widget! {
    Checkbox {
      id: a,
      size: Size::zero(),
      margin if true =>:  EdgeInsets::all(0.),

      SizedBox{
        size: Size::zero(),
        margin: a.margin
      }
    }
  };
}
