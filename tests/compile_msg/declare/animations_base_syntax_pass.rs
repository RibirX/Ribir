use ribir::prelude::*;

fn main() {
  let _def_ref = widget! {
    init ctx => {
      let linear = Transition::declare_builder()
        .easing(easing::LINEAR)
        .build(ctx);
    }
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }

    Animate {
      id: animate1,
      transition: linear,
      prop: prop!(sized_box.size),
      from: Size::new(10., 10.),
    }

    finally {
      let_watch!(sized_box.size)
        .distinct_until_changed()
        .subscribe(move |_| animate1.run());
    }
  };

  let _implicit_from_state = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    transition prop!(sized_box.size) {
      easing: easing::LINEAR,
      duration: std::time::Duration::from_millis(200),
    }

  };

  let _transition_by = widget! {
    init ctx => {
      let linear = transitions::LINEAR.of(ctx);
    }
    SizedBox {
      id: sized_box,
      size: Size::zero()
    }
    transition prop!(sized_box.size) { by: linear }
  };

  let _fix_shorthand_with_builtin_field = widget! {
    SizedBox {
      id: sized_box,
      background: Color::RED,
      size: Size::zero()
    }
    transition prop!(sized_box.background) {
      easing: easing::LINEAR,
      duration: std::time::Duration::from_millis(200),
    }
  };
}
