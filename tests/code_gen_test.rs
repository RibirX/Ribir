use ribir::prelude::*;
use ribir_core::test::*;
use std::{cell::Cell, rc::Rc};

#[test]
fn declare_smoke() {
  let _ = widget! {
    SizedBox {
      size: Size::new(500.,500.),
      background: Color::RED,
     }
  };
}

#[test]
fn simple_ref_bind_work() {
  let size = Size::new(500., 500.);
  let w = widget! {
    Flex {
     SizedBox {
       size: size2.size,
       on_tap: move |_| size2.size *= 2.,
     }
     SizedBox { id: size2, size, }
   }
  };

  let flex_size = Size::new(1000., 500.);
  let mut wnd = Window::default_mock(w, Some(Size::new(2000., 2000.)));
  wnd.layout();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(flex_size));

  tap_at(&mut wnd, (1, 1));

  wnd.layout();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(flex_size * 2.));
}

#[test]
fn event_attr_sugar_work() {
  const BEFORE_SIZE: Size = Size::new(50., 50.);
  const AFTER_TAP_SIZE: Size = Size::new(100., 100.);
  let w = widget! {
    SizedBox {
      id: sized_box,
      size: BEFORE_SIZE,
      SizedBox {
        size: sized_box.size,
        on_tap: move |_| sized_box.size = AFTER_TAP_SIZE,
      }
    }
  };

  let mut wnd = Window::default_mock(w.into_widget(), None);
  wnd.draw_frame();

  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(BEFORE_SIZE));
  assert_layout_result(&wnd, &[0, 0], &ExpectRect::from_size(BEFORE_SIZE));

  tap_at(&mut wnd, (25, 25));

  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(AFTER_TAP_SIZE));
  assert_layout_result(&wnd, &[0, 0], &ExpectRect::from_size(AFTER_TAP_SIZE));
}

#[test]
fn widget_wrap_bind_work() {
  let w = widget! {
    Flex {
      SizedBox {
        id: sibling,
        margin: EdgeInsets::all(1.0),
        size: Size::new(50., 50.),
      }
      SizedBox {
        margin: sibling.margin.clone(),
        size: if sibling.margin.left > 1. { Size::zero() } else { sibling.size },
        on_tap: move |_| sibling.margin = EdgeInsets::all(5.),
      }
    }
  };

  let mut wnd = Window::default_mock(w, Some(Size::new(2000., 2000.)));
  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(104., 52.)));

  tap_at(&mut wnd, (60, 1));

  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(70., 60.)));
}

#[test]
fn expression_for_children() {
  let size_one = Size::new(1., 1.);
  let size_five = Size::new(5., 5.);
  let embed_expr = widget! {
    Flex {
      on_tap: move |_| sized_box.size = size_five,
      SizedBox { id: sized_box, size: size_one }
      // todo: how should we hint user, he/she need wrap inner widget of `DynWidget` to track named widget change.
      DynWidget { dyns: (0..3).map(move |_| widget!{ SizedBox { size: sized_box.size } }) }
      DynWidget {
         dyns: (sized_box.size.area() > 2.).then(|| widget!{ SizedBox { size: sized_box.size } })
      }
    }
  };

  let mut wnd = Window::default_mock(embed_expr, None);
  wnd.layout();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(4., 1.)));
  assert_layout_result(&wnd, &[0, 0], &ExpectRect::from_size(size_one));
  assert_layout_result(&wnd, &[0, 1], &ExpectRect::from_size(size_one));
  assert_layout_result(&wnd, &[0, 2], &ExpectRect::from_size(size_one));
  assert_layout_result(&wnd, &[0, 3], &ExpectRect::from_size(size_one));
  assert_layout_result(&wnd, &[0, 4], &ExpectRect::from_size(ZERO_SIZE));

  tap_at(&mut wnd, (0, 0));
  wnd.layout();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(25., 5.)));
  assert_layout_result(&wnd, &[0, 0], &ExpectRect::from_size(size_five));
  assert_layout_result(&wnd, &[0, 1], &ExpectRect::from_size(size_five));
  assert_layout_result(&wnd, &[0, 2], &ExpectRect::from_size(size_five));
  assert_layout_result(&wnd, &[0, 3], &ExpectRect::from_size(size_five));
  assert_layout_result(&wnd, &[0, 4], &ExpectRect::from_size(size_five));
}

#[test]
fn embed_widget_ref_outside() {
  let w = widget! {
    Flex {
      SizedBox {
        id: first,
        size: Size::new(1., 1.),
        on_tap: move |_| first.size = Size::new(2., 2.)
      }
      DynWidget {
        dyns: (0..3).map(move |_| widget!{ SizedBox { size: first.size } } )
      }
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.layout();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(4., 1.)));

  tap_at(&mut wnd, (0, 0));
  wnd.layout();

  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(Size::new(8., 2.)));
}

#[test]
fn data_flow_macro() {
  let size = Size::new(1., 1.);
  let w = widget! {
    Flex {
      on_tap: move |_| a.size *= 2.,
      SizedBox { id: c, size }
      SizedBox { id: a, size }
      SizedBox { id: b, size: a.size }
    }
    finally {
      watch!(a.size + b.size)
        .subscribe(move |v| c.size = v);
    }
  };
  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();
  let size = layout_size_by_path(&wnd, &[0]);
  // data flow not affect on init.
  assert_eq!(size, Size::new(3., 1.));

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();

  let size = layout_size_by_path(&wnd, &[0]);
  assert_eq!(size, Size::new(8., 4.));
}

#[test]
fn local_var_not_bind() {
  const EXPECT_SIZE: Size = Size::new(5., 5.);
  const BE_CLIPPED_SIZE: Size = Size::new(500., 500.);

  let w = widget! {
    SizedBox {
      size: {
        let _size_box = EXPECT_SIZE;
        let _size_box_def = EXPECT_SIZE;
        _size_box + _size_box_def
      },
      SizedBox {
        id: _size_box,
        size: BE_CLIPPED_SIZE,
      }
    }
  };
  let expect = ExpectRect {
    width: Some(10.),
    height: Some(10.),
    ..<_>::default()
  };
  expect_layout_result(
    w,
    None,
    &[
      LayoutTestItem { path: &[0], expect },
      LayoutTestItem { path: &[0, 0], expect },
    ],
  );
}

#[test]

fn builtin_ref() {
  let icon_track = Rc::new(Cell::new(CursorIcon::default()));
  let c_icon_track = icon_track.clone();

  let w = widget! {
    Flex {
      cursor: tap_box.cursor.clone(),
      SizedBox {
        id: tap_box,
        size: Size::new(5., 5.),
        cursor: CursorIcon::Hand,
        on_tap: move |_| {
          tap_box.cursor.set(CursorIcon::AllScroll);
          c_icon_track.set(tap_box.cursor.get());
        }
      }
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();

  tap_at(&mut wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(icon_track.get(), CursorIcon::AllScroll);
}

#[test]
fn builtin_bind_to_self() {
  let icon_track = Rc::new(Cell::new(CursorIcon::default()));
  let c_icon_track = icon_track.clone();
  let w = widget! {
    SizedBox {
      id: sized_box,
      size: Size::new(5., 5.),
      cursor: {
        let icon = if sized_box.size.area() < 100. {
          CursorIcon::Hand
        } else {
          CursorIcon::Help
        };
        c_icon_track.set(icon);
        icon
      },
      on_tap: move |_|  sized_box.size = Size::new(20.,20.),
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();
  tap_at(&mut wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(icon_track.get(), CursorIcon::Help);
}

fn tap_at(wnd: &mut Window, pos: (u32, u32)) {
  wnd.processes_native_event(WindowEvent::CursorMoved {
    device_id: MockPointerId::zero(),
    position: DevicePoint::new(pos.0, pos.1),
  });
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id: MockPointerId::zero(),
    state: ElementState::Pressed,
    button: MouseButtons::PRIMARY,
  });
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id: MockPointerId::zero(),
    state: ElementState::Released,
    button: MouseButtons::PRIMARY,
  });
}

#[test]
fn builtin_method_support() {
  let layout_size = Stateful::new(Size::zero());
  let w = widget! {
    states { layout_size: layout_size.clone() }
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
    }
    finally{
      watch!(sized_box.layout_size())
        .subscribe(move |v| *layout_size = v);
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();

  assert_eq!(&*layout_size.state_ref(), &Size::new(100., 100.));
}

#[test]
fn fix_builtin_field_can_declare_as_widget() {
  let w = widget! {
    Margin {
      margin: EdgeInsets::all(1.),
      Void {}
    }
  };

  let wnd = Window::default_mock(w, None);
  assert_eq!(wnd.widget_count(), 2);
}

#[test]
fn fix_use_builtin_field_of_builtin_widget_gen_duplicate() {
  let w = widget! {
    Margin {
      id: margin,
      margin: EdgeInsets::all(1.),
      Void {}
    }
    finally {
      watch!(margin.margin.clone()).subscribe(|_| {});
    }
  };

  let wnd = Window::default_mock(w, None);
  assert_eq!(wnd.widget_count(), 2);
}

#[test]
fn fix_access_builtin_with_gap() {
  widget! {
    Void {
      id: this,
      cursor: CursorIcon::Hand,
      on_tap: move |_| {
        // this access cursor across `silent` should compile pass.
        let _ = this.silent().cursor;
      }
    }
  };
}

#[test]
fn fix_subscribe_cancel_after_widget_drop() {
  let notify_cnt = Stateful::new(0);
  let trigger = Stateful::new(true);
  let w = widget! {
    states { cnt: notify_cnt.clone(), trigger: trigger.clone() }
    SizedBox {
      size: Size::zero(),
      DynWidget  {
        dyns: trigger.then(|| {
          widget! {
            SizedBox { size: Size::zero() }
            finally {
              let_watch!(*trigger.deref()).subscribe(move |_| *cnt +=1 );
            }
          }
        })
      }
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();
  {
    *trigger.state_ref() = true
  }
  wnd.draw_frame();
  assert_eq!(*notify_cnt.state_ref(), 1);
  {
    *trigger.state_ref() = true
  }
  wnd.draw_frame();
  assert_eq!(*notify_cnt.state_ref(), 2);
  {
    *trigger.state_ref() = true
  }
  wnd.draw_frame();
  assert_eq!(*notify_cnt.state_ref(), 3);
}

#[test]
fn fix_local_assign_tuple() {
  let w = widget! {
    Row {
      SizedBox {
        id: _sized,
        size: Size::new(1., 1.,),
      }
      SizedBox {
        size: {
          let (x, _) = (_sized, 2);
          x.size
        }
      }
    }
  };

  expect_layout_result(
    w,
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::new(0., 0., 2., 1.),
    }],
  );
}

#[test]
fn fix_silent_not_relayout_dyn_widget() {
  let trigger_size = Stateful::new(ZERO_SIZE);
  let w = widget! {
    states { trigger_size: trigger_size.clone() }
    DynWidget {
      dyns: if trigger_size.area() > 0. {
        SizedBox { size: *trigger_size }
      } else {
        SizedBox { size: ZERO_SIZE }
      }
    }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(ZERO_SIZE));
  {
    *trigger_size.state_ref().silent() = Size::new(100., 100.);
  }
  // after silent modified, dyn widget not rebuild.
  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(ZERO_SIZE));
}

#[test]
fn no_watch() {
  let size = Stateful::new(ZERO_SIZE);
  let w = widget! {
    states { size: size.clone() }
    SizedBox { size: no_watch!(*size) }
  };

  let mut wnd = Window::default_mock(w, None);
  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(ZERO_SIZE));

  {
    *size.state_ref() = Size::new(100., 100.)
  }
  wnd.draw_frame();
  assert_layout_result(&wnd, &[0], &ExpectRect::from_size(ZERO_SIZE));
}

#[test]
fn embed_shadow_states() {
  let _ = widget! {
    // variable `_a` here
    identify(|_a: &BuildCtx| widget! {
      // states shadow `a`
      states { _a: Stateful::new(ZERO_SIZE) }
      // `_a` should be the state `_a`
      SizedBox { size: *_a }
    })
  };
}
