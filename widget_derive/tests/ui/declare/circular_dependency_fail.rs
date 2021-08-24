use ribir::prelude::*;

fn circular_dependency_err() {
  declare! {
    Flex {
      ..<_>::default(),
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
}

fn wrap_widget_circular_err() {
  let _x = declare! {
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
}

fn data_flow_circular_err() {
  let _x = declare! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    data_flow! { a.size ~> a.size }
  };
}

fn data_flow_circular_field_skip_nc_pass() {
  let _x = declare! {
    SizedBox {
      id: a,
      size: Size::zero(),
      SizedBox {
        id: b,
        #[skip_nc]
        size: a.size,
      }
    }
    data_flow! {
      a.size ~> b.size
    }
  };
}

fn circular_follows_with_skip_nc_pass() {
  let _x = declare! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    data_flow! {
      #[skip_nc]
      a.size ~> a.size
    }
  };
}

fn main() {}
