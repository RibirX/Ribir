use ribir::prelude::*;

#[widget]
fn circular_dependency_err(_this: (), ctx: &mut BuildCtx) {
  widget! {
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

#[widget]
fn wrap_widget_circular_err(_this: (), ctx: &mut BuildCtx) {
  widget! {
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

#[widget]
fn data_flow_circular_err(_this: (), ctx: &mut BuildCtx) {
  widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    dataflows { a.size ~> a.size }
  };
}

#[widget]
fn data_flow_circular_field_skip_nc_pass(_this: (), ctx: &mut BuildCtx) {
  widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
      SizedBox {
        id: b,
        #[skip_nc]
        size: a.size,
      }
    }
    dataflows {
      a.size ~> b.size
    }
  };
}

#[widget]
fn circular_follows_with_skip_nc_pass(_this: (), ctx: &mut BuildCtx) {
  widget! {
    SizedBox {
      id: a,
      size: Size::zero(),
    }
    dataflows {
      #[skip_nc]
      a.size ~> a.size
    }
  };
}

fn main() {}
