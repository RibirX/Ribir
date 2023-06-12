use ribir::{core::test_helper::*, prelude::*};
use ribir_dev_helper::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

#[test]
fn declare_smoke() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let _ = widget! {
    SizedBox {
      size: Size::new(500.,500.),
      background: Color::RED,
     }
  };
}

#[test]
fn simple_ref_bind_work() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let size = Size::new(100., 100.);
  let w = widget! {
    Flex {
     SizedBox {
       size: size2.size,
       on_tap: move |_| size2.size *= 2.,
     }
     SizedBox { id: size2, size, }
   }
  };

  let flex_size = Size::new(200., 100.);
  let mut wnd = TestWindow::new(w);
  wnd.layout();
  assert_layout_result_by_path!(wnd, { path = [0], size == flex_size, });

  tap_at(&mut wnd, (1, 1));

  wnd.layout();
  assert_layout_result_by_path!(wnd, { path = [0], size == flex_size * 2., });
}

#[test]
fn event_attr_sugar_work() {
  let _guard = unsafe { AppCtx::new_lock_scope() };
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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  assert_layout_result_by_path!(wnd, { path = [0], size == BEFORE_SIZE, });
  assert_layout_result_by_path!(wnd, { path = [0, 0], size == BEFORE_SIZE, });

  tap_at(&mut wnd, (25, 25));

  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], size == AFTER_TAP_SIZE, });
  assert_layout_result_by_path!(wnd, { path = [0, 0], size == AFTER_TAP_SIZE, });
}

#[test]
fn widget_wrap_bind_work() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], width == 104., height == 52.,});

  tap_at(&mut wnd, (60, 1));

  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], width == 70., height == 60.,});
}

#[test]
fn expression_for_children() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let size_one = Size::new(1., 1.);
  let size_five = Size::new(5., 5.);
  let embed_expr = widget! {
    Flex {
      on_tap: move |_| sized_box.size = size_five,
      SizedBox { id: sized_box, size: size_one }
      // todo: how should we hint user, he/she need wrap inner widget of `DynWidget` to track named widget change.
      Multi::new((0..3).map(move |_| widget!{ SizedBox { size: sized_box.size } }))
      DynWidget {
         dyns: (sized_box.size.area() > 2.).then(|| widget!{ SizedBox { size: sized_box.size } })
      }
    }
  };

  let mut wnd = TestWindow::new(embed_expr);
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], width == 4., height == 1.,});
  assert_layout_result_by_path!(wnd, { path = [0, 0], size == size_one,});
  assert_layout_result_by_path!(wnd, { path = [0, 1], size == size_one,});
  assert_layout_result_by_path!(wnd, { path = [0, 2], size == size_one,});
  assert_layout_result_by_path!(wnd, { path = [0, 3], size == size_one,});
  assert_layout_result_by_path!(wnd, { path = [0, 4], size == ZERO_SIZE,});

  tap_at(&mut wnd, (0, 0));
  wnd.layout();
  assert_layout_result_by_path!(wnd, { path = [0], width == 25., height == 5.,});
  assert_layout_result_by_path!(wnd, { path = [0, 0], size == size_five,});
  assert_layout_result_by_path!(wnd, { path = [0, 1], size == size_five,});
  assert_layout_result_by_path!(wnd, { path = [0, 2], size == size_five,});
  assert_layout_result_by_path!(wnd, { path = [0, 3], size == size_five,});
  assert_layout_result_by_path!(wnd, { path = [0, 4], size == size_five,});
}

#[test]
fn embed_widget_ref_outside() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let w = widget! {
    Flex {
      SizedBox {
        id: first,
        size: Size::new(1., 1.),
        on_tap: move |_| first.size = Size::new(2., 2.)
      }
      Multi::new((0..3).map(move |_| widget!{ SizedBox { size: first.size } }))
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], width == 4., height == 1.,});

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], width == 8., height == 2.,});
}

#[test]
fn data_flow_macro() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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
  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  let size = wnd.layout_info_by_path(&[0]).unwrap().size.unwrap();
  // data flow not affect on init.
  assert_eq!(size, Size::new(3., 1.));

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();

  let size = wnd.layout_info_by_path(&[0]).unwrap().size.unwrap();
  assert_eq!(size, Size::new(8., 4.));
}

fn local_var_not_bind() -> Widget {
  const EXPECT_SIZE: Size = Size::new(5., 5.);
  const BE_CLIPPED_SIZE: Size = Size::new(500., 500.);

  widget! {
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
  }
  .into()
}
widget_layout_test!(
  local_var_not_bind,
  { path = [0], width == 10., height == 10. ,}
  { path = [0, 0], width == 10., height == 10. ,}
);

#[test]

fn builtin_ref() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  tap_at(&mut wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(icon_track.get(), CursorIcon::AllScroll);
}

#[test]
fn builtin_bind_to_self() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  tap_at(&mut wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(icon_track.get(), CursorIcon::Help);
}

fn tap_at(wnd: &mut TestWindow, pos: (i32, i32)) {
  let device_id = unsafe { DeviceId::dummy() };
  let modifiers = ModifiersState::default();

  #[allow(deprecated)]
  wnd.processes_native_event(WindowEvent::CursorMoved {
    device_id,
    position: pos.into(),
    modifiers,
  });
  #[allow(deprecated)]
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id,
    state: ElementState::Pressed,
    button: MouseButton::Left,
    modifiers,
  });
  #[allow(deprecated)]
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id,
    state: ElementState::Released,
    button: MouseButton::Left,
    modifiers,
  });
}

#[test]
fn builtin_method_support() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  assert_eq!(&*layout_size.state_ref(), &Size::new(100., 100.));
}

#[test]
fn fix_builtin_field_can_declare_as_widget() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let w = widget! {
    Margin {
      margin: EdgeInsets::all(1.),
      Void {}
    }
  };

  let wnd = TestWindow::new(w);
  assert_eq!(wnd.widget_count(), 2);
}

#[test]
fn fix_use_builtin_field_of_builtin_widget_gen_duplicate() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let wnd = TestWindow::new(w);
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
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let notify_cnt = Stateful::new(0);
  let trigger = Stateful::new(true);
  let w = widget! {
    states { cnt: notify_cnt.clone(), trigger: trigger.clone() }
    SizedBox {
      size: Size::zero(),
      widget::then(*trigger, || widget! {
        SizedBox { size: Size::zero() }
        finally {
          let_watch!(*trigger).subscribe(move |_| *cnt +=1 );
        }
      })
    }
  };

  let mut wnd = TestWindow::new(w);
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

fn fix_local_assign_tuple() -> Widget {
  widget! {
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
  }
  .into()
}
widget_layout_test!(
  fix_local_assign_tuple,
  rect == ribir_geom::rect(0., 0., 2., 1.),
);

#[test]
fn fix_silent_not_relayout_dyn_widget() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

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

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], size == ZERO_SIZE,});
  {
    *trigger_size.state_ref().silent() = Size::new(100., 100.);
  }
  // after silent modified, dyn widget not rebuild.
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], size == ZERO_SIZE,});
}

#[test]
fn no_watch() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let size = Stateful::new(ZERO_SIZE);
  let w = widget! {
    states { size: size.clone() }
    SizedBox { size: no_watch!(*size) }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], size == ZERO_SIZE,});

  {
    *size.state_ref() = Size::new(100., 100.)
  }
  wnd.draw_frame();
  assert_layout_result_by_path!(wnd, { path = [0], size == ZERO_SIZE,});
}

#[test]
fn embed_shadow_states() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let _ = widget! {
    // variable `_a` here
    FnWidget::new(|_: &BuildCtx| widget! {
      // states shadow `a`
      states { _a: Stateful::new(ZERO_SIZE) }
      // `_a` should be the state `_a`
      SizedBox { size: *_a }
    })
  };
}

#[test]
fn untrack_prop_with_pure_lambda() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let trigger = Stateful::new(0_u32);
  let counter = Stateful::new(0_u32);
  let w = widget! {
    states {
      trigger: trigger.clone(),
      counter: counter.clone()
    }
    SizedBox {
      id: host,
      size: Size::new(50., 50.),
    }
    Animate {
      id: animate,
      transition: Transition {
        delay:  None,
        duration: Duration::from_millis(150),
        easing: easing::EASE_IN,
        repeat: None,
      },
      prop: prop!(host.size, move |from, to, rate| {
        let _ = *trigger;
        to.lerp(from, rate)
      }),
      from: Size::zero(),
    }
    finally {
      let_watch!(animate.from)
        .subscribe(move |_| {
          *counter += 1;
        });
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  assert_eq!(*counter.state_ref(), 0);
  *trigger.state_ref() += 1;
  wnd.draw_frame();

  assert_eq!(*counter.state_ref(), 0);
}

#[test]
fn embed_widget() {
  let _guard = unsafe { AppCtx::new_lock_scope() };

  let _ = widget! {
    Column {
      widget! { // simple case & as first child
        Container {
          size: Size::new(10., 10.),
        }
      }
      Container {
        size: Size::zero(),
      }
      widget! { // all feature & as middle child
        init ctx => {
          let linear_transition = transitions::LINEAR.of(ctx);
        }
        Row {
          SizedBox {
            id: sized_box,
            size: Size::new(100., 100.),
            on_tap: move |_| {
              leak_animate.run();
            },
          }
        }
        Animate {
          id: leak_animate,
          transition: linear_transition,
          prop: prop!(sized_box.size),
          from: ZERO_SIZE,
        }
        finally {
          watch!(leak_animate.is_running())
            .subscribe(move |v| println!("{v}"));
        }
      }
      Container {
        size: Size::zero(),
      }
      widget! {  // embed multi widget! & as last child
        Row {
          widget! {
            Container {
              size: Size::zero(),
            }
          }
        }
      }
    }
  };
}
