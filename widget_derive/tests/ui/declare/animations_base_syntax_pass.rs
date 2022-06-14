use ribir::prelude::*;

fn main() {
  let _def_ref = widget! {
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

  let _standard = widget! {
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

  let _implicit_from_state = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      sized_box.size: Transition { },
    }
  };
}
