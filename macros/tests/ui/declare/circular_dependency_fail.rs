use ribir::prelude::*;

fn main() {
  let _circular_dependency_err = widget! {
    Flex {
      SizedBox {
        id: id1,
        size: id2.size,
      }
      SizedBox {
        id: id2,
        size: id3.size,
      }
      SizedBox {
        id: id3,
        size: id1.size,
      }
    }
  };

  let _wrap_widget_circular_err = widget! {
    SizedBox {
      id: parent,
      size: Size::zero(),
      margin: child.margin.clone(),
      SizedBox{
        id: child,
        size: Size::zero(),
        margin: parent.margin.clone(),
      }
    }
  };

  let _data_flow_circular_err = widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    on a.size ~> a.size
  };

  let _data_flow_circular_field_skip_nc_pass = widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
      SizedBox {
        id: b,
        #[skip_nc]
        size: a.size,
      }
    }
    on a.size ~> b.size
  };

  let _circular_follows_with_skip_nc_pass = widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    #[skip_nc]
    on a.size ~> a.size
  };
}
