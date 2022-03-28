use ribir::prelude::*;

fn main() {}

#[widget]
fn ref_parent(_this: (), ctx: &mut BuildCtx) {
  widget! {
    SizedBox {
      id: size_box,
      size: Size::new(50., 50.),
      SizedBox {
        size: size_box.size,
      }
    }
  };
}

#[widget]
fn ref_child(_this: (), ctx: &mut BuildCtx) {
  widget! {
     SizedBox {
       size: child_box.size,
       SizedBox {
        id: child_box,
        size: Size::new(50., 50.),
      }
     }
  };
}

#[widget]
fn ref_sibling(_this: (), ctx: &mut BuildCtx) {
  widget! {
    Flex {
      SizedBox {
        size: size2.size,
      }
      SizedBox {
       id: size2,
       size: size3.size,
     }
     SizedBox {
      id: size3,
      size: Size::new(1., 1.),
    }
    }
  };
}

#[widget]
fn temp_var_name_not_conflict(_this: (), ctx: &mut BuildCtx) {
  widget! {
    Flex {
      SizedBox {
        id: c0,
        size: w.size,
      }
      SizedBox {
        id: w,
        size:  Size::new(500., 500.),
      }
      SizedBox {
        size: c0.size,
      }
    }
  };
}

#[widget]
fn wrap_widget_effect_order(_this: (), ctx: &mut BuildCtx) {
  let _x = widget! {
    SizedBox {
      size: Size::zero(),
      margin: child.margin.clone(),
      SizedBox{
        id: child,
        size: Size::zero(),
        margin: EdgeInsets::all(1.),
      }
    }
  };
}
