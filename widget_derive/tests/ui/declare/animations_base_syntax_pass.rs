use ribir::prelude::*;

fn main() {
  let _def_ref = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      sized_box.size: Animate {
        id: animate1,
        from: State {
          sized_box.size: Size::new(10., 10.),
        },
        transition:  Transition {
          id: transition1,
          easing: easing::LINEAR
        },
      }
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
        transition: Transition {
          easing: easing::LINEAR
        }
      }
    }
  };

  let _implicit_from_state = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    animations {
      sized_box.size: Transition { easing: easing::LINEAR },
    }
  };

  let _fix_shorthand_with_builtin_field = widget! {
    SizedBox {
      id: sized_box,
      background: Color::RED,
      size: Size::zero()
    }
    animations {
      sized_box.background: Transition { easing: easing::LINEAR },
    }
  };

  let _state_field_shorthand = widget! {
    SizedBox { id: sized_box, size: Size::zero() }
    animations {
      sized_box.size: Animate {
        from: State { sized_box.size },
        transition: Transition { easing: easing::LINEAR }
      },
    }
  };
  let _default_from_state = widget! {
    SizedBox { id: sized_box, size: Size::zero() }
    animations {
      sized_box.size: Animate {
        transition: Transition { easing: easing::LINEAR }
      },
    }
  };
}
