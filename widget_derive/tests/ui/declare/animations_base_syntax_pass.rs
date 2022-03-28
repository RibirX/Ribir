use ribir::prelude::*;

#[widget]
fn def_ref(_this: (), ctx: &mut BuildCtx) {
  let _ = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      State {
        id: state1,
        sized_box.size: Size::new(10., 10.),
      }
      Transition {
        id: transition1,
      }
      Animate {
        id: animate1,
        from: state1,
        transition: transition1,
      }
      sized_box.size: animate1
    }
  };
}

#[widget]
fn standard(_this: (), ctx: &mut BuildCtx) {
  let _ = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      sized_box.size: Animate {
        from: State { sized_box.size: Size::new(10., 10.) },
        transition: Transition { }
      }
    }
  };
}

#[widget]
fn implicit_from_state(_this: (), ctx: &mut BuildCtx) {
  let _ = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      sized_box.size: Transition { },
    }
  };
}

fn main() {}
