use ribir::prelude::*;

fn fields_no_follow_with_skip_nc(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
      id: self_id,
      #[skip_nc]
      size: Size::new(5., 5.),
    }
  };
}

fn attrs_no_follow_with_skip_nc(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
      id: self_id,
      size: Size::new(5., 5.),
      #[skip_nc]
      cursor: CursorIcon::Help,
    }
  };
}

fn wrap_widget_no_follow_with_skip_nc(ctx: &mut BuildCtx) {
  declare! {
    SizedBox {
      id: self_id,
      size: Size::new(5., 5.),
      #[skip_nc]
      margin: Margin::all(1.)
    }
  };
}

fn main() {}
