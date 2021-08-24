use ribir::prelude::*;

fn main() {}

fn ref_parent() {
  declare! {
    SizedBox {
      id: size_box,
      size: Size::new(50., 50.),
      SizedBox {
        size: size_box.size,
      }
    }
  };
}

fn ref_child() {
  declare! {
     SizedBox {
       size: child_box.size,
       SizedBox {
        id: child_box,
        size: Size::new(50., 50.),
      }
     }
  };
}

fn ref_sibling() {
  declare! {
    Flex {
      ..<_>::default(),
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

fn temp_var_name_not_conflict() {
  declare! {
    Flex {
      ..<_>::default(),
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

fn wrap_widget_effect_order() {
  let _x = declare! {
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
