use ribir::prelude::*;

fn main() {
  compile_error!("Test for declare syntax warning.");
  let _unused_id_warning = widget! {
    SizedBox {
      id: test_id,
      size: Size::zero()
    }
  };
  let _used_id_no_warning = widget! {
    SizedBox {
      id: id1,
      size: Size::new(100., 100.),
      SizedBox {
        size: id1.size,
      }
    }
  };

  let _animate_used_builtin_no_warning = widget! {
    SizedBox {
      id: id1,
      size: Size::zero(),
      background: Color::RED,
    }
    animations {
      id1.tap: Animate {
        from: State { id1.background },
        transition: Transition { easing: easing::LINEAR }
      }
    }
  };

  let _fix_use_no_declared_builtin_no_warning = widget! {
    SizedBox {
      id: sized_box,
      size: Size::zero(),
      SizedBox { size: Size::zero(), background: sized_box.background }
    }
  };
}
