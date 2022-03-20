use ribir::prelude::*;

fn def_ref(ctx: &mut BuildCtx) {
  let _ = declare! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      State {
        id: state1,
        sized_box.size: Size::new(10, 10),
      }
      Transition {
        id: transition1,
        delay: 50,
      }
      Animate {
        id: animate1,
        from: state1,
        transition: transition1,
      }
      state1.size: animate1
    }
  };
}

fn standard(ctx: &mut BuildCtx) {
  let _ = declare! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      state1.size: Animate {
        from: State { sized_box.size: Size::new(10, 10) },
        transition: Transition { delay: 50 }
      }
    }
  };
}

fn implicit_from_state(ctx: &mut BuildCtx) {
  let _ = declare! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      state1.size: Transition { delay: 50 },
    }
  };
}

fn main() {}
