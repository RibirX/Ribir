use ribir::prelude::*;

fn main() {
  let _ref_parent = widget! {
    SizedBox {
      id: size_box,
      size: Size::new(50., 50.),
      SizedBox {
        size: size_box.size,
      }
    }
  };

  let _ref_child = widget! {
      SizedBox {
        size: child_box.size,
        SizedBox {
          id: child_box,
          size: Size::new(50., 50.),
        }
     }
  };

  let _ref_sibling = widget! {
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

  let _temp_var_name_not_conflict = widget! {
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

  let _wrap_widget_effect_order = widget! {
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
