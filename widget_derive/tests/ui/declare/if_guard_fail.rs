use ribir::prelude::*;

fn normal_if_guard_pass() {
  let guard = Some(1);
  let _ = declare! {
    SizedBox {
      // if guard in widget's field
      size if true => : Size::zero(),
      // if guard in data atribute
      cursor if true => : CursorIcon::Hand,
      // if guard in listener attibute
      on_tap if let Some(_) = guard  => : |_| {},
      // if guard in wrap widget.
      margin if true => : EdgeInsets::all(1.)
    }
  };
}

fn id_if_guard_fail() {
  let guard = Some(1);
  let _ = declare! {
    SizedBox {
      id if true => : test,
      // if guard in widget's field
      size if true => : Size::zero(),

    }
  };
}

fn depend_id_behind_if_guard_fail() {
  let _ = declare! {
    SizedBox {
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

fn main() {}
