use ribir::prelude::*;

fn main() {
  let _fields_no_follow_with_skip_nc = widget! {
    SizedBox {
      id: self_id,
      #[skip_nc]
      size: Size::new(5., 5.),
    }
  };

  let _attrs_no_follow_with_skip_nc = widget! {
    SizedBox {
      id: self_id,
      size: Size::new(5., 5.),
      #[skip_nc]
      cursor: CursorIcon::Help,
    }
  };

  let _wrap_widget_no_follow_with_skip_nc = widget! {
    SizedBox {
      id: self_id,
      size: Size::new(5., 5.),
      #[skip_nc]
      margin: EdgeInsets::all(1.)
    }
  };

  compile_error!("Test for declare syntax warning.");
}
