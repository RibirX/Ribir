use ribir::{
  core::{reset_test_env, test_helper::*},
  prelude::*,
};
use ribir_dev_helper::*;
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

widget_layout_test!(
  simplest_leaf_rdl,
  WidgetTester::new(fn_widget! {
    rdl!{ SizedBox { size: Size::new(500.,500.) } }
  }),
  LayoutCase::default().with_size(Size::new(500., 500.))
);

widget_layout_test!(
  with_child_rdl,
  WidgetTester::new(fn_widget! {
    rdl!{
      Row {
        rdl!{ SizedBox { size: Size::new(500.,500.)  } }
      }
    }
  }),
  LayoutCase::default().with_size(Size::new(500., 500.))
);

widget_layout_test!(
  with_builtin_child_rdl,
  WidgetTester::new(fn_widget! {
    rdl!{ SizedBox {
      size: Size::new(500.,500.),
      margin: EdgeInsets::all(10.)
    }}
  }),
  LayoutCase::default().with_size(Size::new(520., 520.))
);

widget_layout_test!(
  rdl_with_child,
  WidgetTester::new(fn_widget! {
    let single_p = rdl!{ SizedBox { size: Size::new(500.,500.)  }};
    rdl!{ $single_p { rdl!{ Void } } }
  }),
  LayoutCase::default().with_size(Size::new(500., 500.))
);

widget_layout_test!(
  single_rdl_has_builtin_with_child,
  WidgetTester::new(fn_widget! {
    let single_p = rdl!{ SizedBox {
      size: Size::new(500.,500.),
      margin: EdgeInsets::all(10.)
    }};
    rdl!{ $single_p { rdl!{ Void } } }
  }),
  LayoutCase::default().with_size(Size::new(520., 520.))
);

widget_layout_test!(
  multi_child_rdl_has_builtin_with_child,
  WidgetTester::new(fn_widget! {
    let multi_p = rdl!{ Flex {
      margin: EdgeInsets::all(10.)
    } };
    rdl!{ $multi_p { rdl!{ Void } } }
  }),
  LayoutCase::default().with_size(Size::new(20., 20.))
);

widget_layout_test!(
  compose_child_rdl_has_builtin_with_child,
  WidgetTester::new(fn_widget! {
    let multi_p = rdl!{ Row { margin: EdgeInsets::all(10.) }};
    rdl!{ $multi_p { rdl!{ Void {} }} }
  }),
  LayoutCase::default().with_size(Size::new(20., 20.))
);

widget_layout_test!(
  access_rdl_widget,
  WidgetTester::new(fn_widget! {
    let b = rdl!{ SizedBox {size: Size::new(500.,500.)}};
    rdl!{ Row {
      rdl!{ SizedBox { size: $b.size } }
      rdl!{ b }
    }}
  }),
  LayoutCase::default().with_size(Size::new(1000., 500.))
);

widget_layout_test!(
  access_builtin_rdl_widget,
  WidgetTester::new(fn_widget! {
    let mut b = rdl!{ SizedBox {
      size: Size::new(100.,100.),
      margin: EdgeInsets::all(10.)
    }};

    rdl!{
      Row {
        rdl!{
          SizedBox {
            size: $b.size,
            margin: $b.margin,
          }
        }
        rdl!{ b }
      }
    }
  }),
  LayoutCase::default().with_size(Size::new(240., 120.))
);

widget_layout_test!(
  dollar_as_rdl_parent,
  WidgetTester::new(fn_widget! {
    let b = rdl!{SizedBox { size: Size::new(500.,500.) }};
    rdl!{ $b { rdl!{ Void {}} } }
  }),
  LayoutCase::default().with_size(Size::new(500., 500.))
);

widget_layout_test!(
  dollar_as_middle_parent,
  WidgetTester::new(fn_widget! {
    let b = rdl!{ SizedBox { size: Size::new(500.,500.) }};
    rdl!{ Row { rdl!{ $b { rdl!{ Void {} } } } } }
  }),
  LayoutCase::default().with_size(Size::new(500., 500.))
);

widget_layout_test!(
  pipe_as_field_value,
  WidgetTester::new({
    let (size, w_size) = split_value(Size::zero());
    let w = fn_widget! {
      rdl!{ SizedBox { size: pipe!(*$size) }}
    };
    *w_size.write() = Size::new(100., 100.);
    w
  }),
  LayoutCase::default().with_size(Size::new(100., 100.))
);

widget_layout_test!(
  pipe_as_builtin_field_value,
  WidgetTester::new({
    let (margin, w_margin) = split_value(EdgeInsets::all(0.));
    let w = fn_widget! {
      rdl!{ SizedBox {
        size: Size::zero(),
        margin: pipe!(*$margin)
      }}
    };
    *w_margin.write() = EdgeInsets::all(50.);

    w
  }),
  LayoutCase::default().with_size(Size::new(100., 100.))
);

widget_layout_test!(
  pipe_with_ctx,
  WidgetTester::new({
    let (scale, w_scale) = split_value(1.);
    let w = fn_widget! {
      rdl!{ SizedBox {
        size: pipe!(IconSize::of(BuildCtx::get()).tiny * *$scale)
      }}
    };
    *w_scale.write() = 2.;
    w
  }),
  LayoutCase::default().with_size(Size::new(36., 36.))
);

widget_layout_test!(
  pipe_with_builtin_field,
  WidgetTester::new(fn_widget! {
    let mut box1 = @SizedBox { size: Size::zero(), margin: EdgeInsets::all(1.) };
    let box2 = @SizedBox { size: $box1.size, margin: pipe!($box1.margin) };
    @Row {
      @{ box1 }
      @{ box2 }
    }
  }),
  LayoutCase::default().with_size(Size::new(4., 2.))
);

widget_layout_test!(
  capture_closure_used_ctx,
  WidgetTester::new(fn_widget! {
    let size_box = @SizedBox { size: ZERO_SIZE };
    @ $size_box {
      on_mounted: move |e| $size_box.write().size = IconSize::of(&e).tiny
    }
  }),
  LayoutCase::default().with_size(Size::new(18., 18.))
);

#[test]
fn pipe_single_parent() {
  reset_test_env!();

  let outside_blank = Stateful::new(true);
  let outside_blank2 = outside_blank.clone_writer();
  let w = fn_widget! {
    let edges = EdgeInsets::all(5.);
    let blank = pipe! {
      if *$outside_blank {
        BoxedSingleChild::new(Margin { margin: edges })
      } else {
        BoxedSingleChild::new(FittedBox::new(BoxFit::None))
      }
    };
    rdl!{
      $blank {
        rdl!{ SizedBox { size: Size::new(100., 100.) } }
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(110., 110.));

  *outside_blank2.write() = false;
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(100., 100.));
}

#[test]
fn pipe_multi_parent() {
  reset_test_env!();

  let stack_or_flex = Stateful::new(true);
  let stack_or_flex2 = stack_or_flex.clone_writer();
  let w = fn_widget! {
    let container = pipe! {
      if *$stack_or_flex {
        BoxedMultiChild::new(rdl!{ Stack { } })
      } else {
        BoxedMultiChild::new(rdl!{ Flex { } })
      }
    };

    rdl!{
      $container {
        rdl!{ SizedBox { size: Size::new(100., 100.) } }
        rdl!{ SizedBox { size: Size::new(100., 100.) } }
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size((100., 100.).into());

  *stack_or_flex2.write() = false;
  wnd.draw_frame();
  wnd.assert_root_size((200., 100.).into());
}

#[test]
fn pipe_as_child() {
  reset_test_env!();

  let box_or_not = Stateful::new(true);
  let box_or_not2 = box_or_not.clone_watcher();
  let w = fn_widget! {
    let blank = pipe!{
      $box_or_not2.then(|| rdl!{ SizedBox { size: Size::new(100., 100.) } })
    };
    rdl!{ Stack { rdl!{ blank } } }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size((100., 100.).into());

  *box_or_not.write() = false;

  wnd.draw_frame();
  wnd.assert_root_size(Size::zero());
}

#[test]
fn pipe_as_multi_child() {
  reset_test_env!();
  let (cnt, w_cnt) = split_value(0);

  let w = fn_widget! {
    let fix_box = SizedBox { size: Size::new(100., 100.) };
    let boxes = pipe! {
      (0..*$cnt).map(|_| fix_box.clone()).collect::<Vec<_>>()
    };
    rdl!{ Flex { rdl!{ boxes } } }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(Size::zero());

  *w_cnt.write() = 3;
  wnd.draw_frame();
  wnd.assert_root_size((300., 100.).into());
}

widget_layout_test!(
  at_in_widget_macro,
  WidgetTester::new(fn_widget! {
    @SizedBox { size: Size::new(100., 100.) }
  }),
  LayoutCase::default().with_size(Size::new(100., 100.))
);

widget_layout_test!(
  at_as_variable_in_widget,
  WidgetTester::new(fn_widget! {
    let size = Size::new(100., 100.);
    let row = @Row {};
    @ $row {
      // @ in @
      @SizedBox { size }
      // `rdl!` in @
      rdl!{ SizedBox { size } }
    }
  }),
  LayoutCase::default().with_size(Size::new(200., 100.))
);

widget_layout_test!(
  at_as_variable_in_rdl,
  WidgetTester::new(fn_widget! {
    let size = Size::new(100., 100.);
    let row = @Row {};
    rdl!{
      $row {
        @SizedBox { size }
        @SizedBox { size }
      }
    }
  }),
  LayoutCase::default().with_size(Size::new(200., 100.))
);

widget_layout_test!(
  access_builtin_field_by_dollar,
  WidgetTester::new(fn_widget! {
    let size = Size::new(100., 100.);
    let mut box1 = @SizedBox { size, margin: EdgeInsets::all(10.) };
    let box2 = @SizedBox { size, margin: $box1.margin };
    @Row { @ { box1 } @{ box2 } }
  }),
  LayoutCase::default().with_size(Size::new(240., 120.))
);

#[test]
fn closure_in_fn_widget_capture() {
  reset_test_env!();

  let hi_res = Stateful::new(CowArc::borrowed(""));
  let hi_res2 = hi_res.clone_reader();
  let w = fn_widget! {
    let text = @ Text { text: "hi" };
    let on_mounted = move |_: &mut _| *$hi_res.write() =$text.text.clone();
    @ $text { on_mounted }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  assert_eq!(&**hi_res2.read(), "hi");
}

widget_layout_test!(
  at_embed_in_expression,
  WidgetTester::new(fn_widget! {
    @Row {
      @{ (0..3).map(|_| {
        @SizedBox { size: Size::new(100., 100.) }
      })}
    }
  }),
  LayoutCase::default().with_size(Size::new(300., 100.))
);

#[test]
fn declare_smoke() {
  reset_test_env!();

  let _ = fn_widget! {
    @SizedBox {
      size: Size::new(500.,500.),
      background: Color::RED,
    }
  };
}

#[test]
fn simple_ref_bind_work() {
  reset_test_env!();

  let size = Size::new(100., 100.);
  let w = fn_widget! {
    let size2 = @SizedBox { size };
    @Flex {
     @SizedBox {
       size: pipe!($size2.size),
       on_tap: move |_| $size2.write().size *= 2.,
     }
     @ { size2 }
   }
  };

  let flex_size = Size::new(200., 100.);
  let mut wnd = TestWindow::new(w);
  wnd.layout();
  wnd.assert_root_size(flex_size);

  tap_at(&wnd, (1, 1));

  wnd.draw_frame();
  wnd.assert_root_size(flex_size * 2.);
}

#[test]
fn event_attr_sugar_work() {
  reset_test_env!();
  const BEFORE_SIZE: Size = Size::new(50., 50.);
  const AFTER_TAP_SIZE: Size = Size::new(100., 100.);
  let w = fn_widget! {
    let sized_box = @SizedBox { size: BEFORE_SIZE };
    @$sized_box {
      @SizedBox {
        size: pipe!($sized_box.size),
        on_tap: move |_| $sized_box.write().size = AFTER_TAP_SIZE,
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  wnd.assert_root_size(BEFORE_SIZE);
  LayoutCase::expect_size(&wnd, &[0, 0], BEFORE_SIZE);

  tap_at(&wnd, (25, 25));

  wnd.draw_frame();
  wnd.assert_root_size(AFTER_TAP_SIZE);
  LayoutCase::expect_size(&wnd, &[0, 0], AFTER_TAP_SIZE);
}

#[test]
fn widget_wrap_bind_work() {
  reset_test_env!();

  let w = fn_widget! {
    let mut sibling = @SizedBox {
      margin: EdgeInsets::all(1.0),
      size: Size::new(50., 50.),
    };
    let next_box = @SizedBox {
      margin: pipe!($sibling.margin),
      size: pipe!(if $sibling.margin.left > 1. { Size::zero() } else { $sibling.size }),
      on_tap: move |_| $sibling.write().margin = EdgeInsets::all(5.),
    };
    @Flex {
      @ { [sibling, next_box ] }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(104., 52.));
  tap_at(&wnd, (60, 1));

  wnd.draw_frame();
  wnd.assert_root_size(Size::new(70., 60.));
}

#[test]
fn expression_for_children() {
  reset_test_env!();

  let size_one = Size::new(1., 1.);
  let size_five = Size::new(5., 5.);
  let embed_expr = fn_widget! {
    let sized_box = @SizedBox { size: size_one };
    let multi_box = (0..3).map(move |_| @SizedBox { size: pipe!($sized_box.size) });
    let pipe_box = pipe!($sized_box.size.area() > 2.)
      .map(move |v| v.then(|| @SizedBox { size: pipe!($sized_box.size) }));

    @Flex {
      on_tap: move |_| $sized_box.write().size = size_five,
      @ { sized_box }
      @ { multi_box }
      @ { pipe_box }
    }
  };

  let mut wnd = TestWindow::new(embed_expr);
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(4., 1.));
  LayoutCase::expect_size(&wnd, &[0, 0], size_one);
  LayoutCase::expect_size(&wnd, &[0, 1], size_one);
  LayoutCase::expect_size(&wnd, &[0, 2], size_one);
  LayoutCase::expect_size(&wnd, &[0, 3], size_one);
  LayoutCase::expect_size(&wnd, &[0, 4], ZERO_SIZE);

  tap_at(&wnd, (0, 0));
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(25., 5.));
  LayoutCase::expect_size(&wnd, &[0, 0], size_five);
  LayoutCase::expect_size(&wnd, &[0, 1], size_five);
  LayoutCase::expect_size(&wnd, &[0, 2], size_five);
  LayoutCase::expect_size(&wnd, &[0, 3], size_five);
  LayoutCase::expect_size(&wnd, &[0, 4], size_five);
}

#[test]
fn embed_widget_ref_outside() {
  reset_test_env!();

  let w = fn_widget! {
    let first = @SizedBox { size: Size::new(1., 1.) };
    let three_box = @{ (0..3).map(move |_| @ SizedBox { size: pipe!($first.size) } )};
    @Flex {
      @$first { on_tap: move |_| $first.write().size = Size::new(2., 2.)}
      @{ three_box }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(4., 1.));

  tap_at(&wnd, (0, 0));
  wnd.draw_frame();
  wnd.assert_root_size(Size::new(8., 2.));
}

#[test]
fn bind_fields() {
  reset_test_env!();

  let size = Size::new(1., 1.);
  let w = fn_widget! {
    let a = @SizedBox { size };
    let b = @SizedBox { size: pipe!($a.size) };
    let c = @SizedBox { size };
    watch!($a.size + $b.size)
      .subscribe(move |v| $c.write().size = v);
    @Flex {
      on_tap: move |_| $a.write().size *= 2.,
      @ { [a, b, c] }
    }
  };
  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  let size = wnd
    .layout_info_by_path(&[0])
    .unwrap()
    .size
    .unwrap();
  assert_eq!(size, Size::new(4., 2.));

  tap_at(&wnd, (0, 0));
  wnd.draw_frame();

  let size = wnd
    .layout_info_by_path(&[0])
    .unwrap()
    .size
    .unwrap();
  assert_eq!(size, Size::new(8., 4.));
}

const EXPECT_SIZE: Size = Size::new(5., 5.);
const BE_CLIPPED_SIZE: Size = Size::new(500., 500.);

widget_layout_test!(
  local_var_not_bind,
  WidgetTester::new(fn_widget! {
    let _size_box = @SizedBox { size: BE_CLIPPED_SIZE };
    @SizedBox {
      size: {
        let _size_box = EXPECT_SIZE;
        let _size_box_def = EXPECT_SIZE;
        _size_box + _size_box_def
      },
      @{ _size_box }
    }
  }),
  LayoutCase::default().with_size(Size::new(10., 10.)),
  LayoutCase::new(&[0, 0]).with_size(Size::new(10., 10.))
);

#[test]

fn builtin_ref() {
  reset_test_env!();

  let (icon, w_icon) = split_value(CursorIcon::default());

  let w = fn_widget! {
    let mut tap_box = @SizedBox {
      size: Size::new(5., 5.),
      cursor: CursorIcon::Pointer,
    };
    @Flex {
      cursor: pipe!($tap_box.cursor),
      @$tap_box {
        on_tap: move |_| {
          $tap_box.write().cursor = CursorIcon::AllScroll;
          *$w_icon.write() = $tap_box.cursor;
        }
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  tap_at(&wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(*icon.read(), CursorIcon::AllScroll);
}

#[test]
fn builtin_bind_to_self() {
  reset_test_env!();

  let (icon, w_icon) = split_value(CursorIcon::default());
  let w = fn_widget! {
    let w_icon = w_icon.clone_writer();
    let sized_box = @SizedBox { size: Size::new(5., 5.) };
    @$sized_box {
      cursor: pipe!{
        let icon = if $sized_box.size.area() < 100. {
          CursorIcon::Pointer
        } else {
          CursorIcon::Help
        };
        *w_icon.silent() = icon;
        icon
      },
      on_tap: move |_| $sized_box.write().size = Size::new(20.,20.),
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  tap_at(&wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(*icon.read(), CursorIcon::Help);
}

fn tap_at(wnd: &TestWindow, pos: (i32, i32)) {
  let device_id = unsafe { DeviceId::dummy() };

  #[allow(deprecated)]
  wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: pos.into() });

  wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);

  wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
}

#[test]
fn builtin_method_support() {
  reset_test_env!();

  let layout_size = Stateful::new(Size::zero());
  let c_layout_size = layout_size.clone_reader();
  let w = fn_widget! {
    let mut sized_box  = @SizedBox { size: Size::new(100., 100.) };
    watch!($sized_box.layout_size())
      .subscribe(move |v| *$layout_size.write() = v);
    sized_box
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();

  assert_eq!(*c_layout_size.read(), Size::new(100., 100.));
}

#[test]
fn fix_builtin_field_can_declare_as_widget() {
  reset_test_env!();

  let w = fn_widget! {
    @Margin {
      margin: EdgeInsets::all(1.),
      @Void {}
    }
  };

  let wnd = TestWindow::new(w);

  assert_eq!(wnd.content_count(), 2);
}

#[test]
fn fix_use_builtin_field_of_builtin_widget_gen_duplicate() {
  reset_test_env!();

  let w = fn_widget! {
    let mut margin = @Margin { margin: EdgeInsets::all(1.) };
    watch!($margin.margin).subscribe(|_| {});
    @$margin  { @Void {} }
  };

  let wnd = TestWindow::new(w);
  assert_eq!(wnd.content_count(), 2);
}

#[test]
fn fix_access_builtin_with_gap() {
  let _ = fn_widget! {
    let mut this = @Void { cursor: CursorIcon::Pointer };
    @$this {
      on_tap: move |_| {
        // this access cursor across `silent` should compile pass.
        let _ = $this.silent().cursor;
      }
    }
  };
}

#[test]
fn fix_subscribe_cancel_after_widget_drop() {
  reset_test_env!();

  let (cnt, w_cnt) = split_value(0);
  let (trigger, w_trigger) = split_value(true);
  let w = fn_widget! {
    let mut container = @SizedBox { size: Size::zero() };
    let h = watch!(*$trigger).subscribe(move |_| *$w_cnt.write() +=1 );
    container = container.on_disposed(move |_| h.unsubscribe());

    @$container {
      @ {
        pipe!{$trigger.then(|| {
          @SizedBox { size: Size::zero() }
        })}
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  assert_eq!(*cnt.read(), 1);
  *w_trigger.write() = true;
  wnd.draw_frame();
  assert_eq!(*cnt.read(), 2);
  *w_trigger.write() = true;
  wnd.draw_frame();
  assert_eq!(*cnt.read(), 3);
  *w_trigger.write() = true;
  wnd.draw_frame();
  assert_eq!(*cnt.read(), 4);
}

widget_layout_test!(
  fix_local_assign_tuple,
  WidgetTester::new(fn_widget! {
    let _sized = @SizedBox { size: Size::new(1., 1.) };
    let sized_box2 = @SizedBox {
      size: {
        let (x, _) = ($_sized, 2);
        x.size
      }
    };
    @Row {
      @ { _sized }
      @ { sized_box2 }
    }
  }),
  LayoutCase::default().with_rect(ribir_geom::rect(0., 0., 2., 1.))
);

#[test]
fn fix_silent_not_relayout_dyn_widget() {
  reset_test_env!();

  let trigger_size = Stateful::new(ZERO_SIZE);
  let c_trigger_size = trigger_size.clone_writer();
  let w = fn_widget! {
    pipe! {
      if $trigger_size.area() > 0. {
        SizedBox { size: *$trigger_size }
      } else {
        SizedBox { size: ZERO_SIZE }
      }
    }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(ZERO_SIZE);
  {
    *c_trigger_size.silent() = Size::new(100., 100.);
  }
  // after silent modified, dyn widget not rebuild.
  wnd.draw_frame();
  wnd.assert_root_size(ZERO_SIZE);
}

#[test]
fn no_watch() {
  reset_test_env!();

  let size = Stateful::new(ZERO_SIZE);
  let c_size = size.clone_reader();
  let w = fn_widget! {
    @SizedBox { size: *$c_size }
  };

  let mut wnd = TestWindow::new(w);
  wnd.draw_frame();
  wnd.assert_root_size(ZERO_SIZE);

  {
    *size.write() = Size::new(100., 100.)
  }
  wnd.draw_frame();
  wnd.assert_root_size(ZERO_SIZE);
}

#[test]
fn fix_direct_use_map_writer_with_builtin() {
  fn _x(mut host: FatObj<Void>, ctx!(): &BuildCtx) {
    let _anchor = host
      .get_relative_anchor_widget()
      .map_writer(|w| PartData::from_ref_mut(&mut w.anchor));
    let _anchor = host
      .get_relative_anchor_widget()
      .map_writer(|w| PartData::from_ref_mut(&mut w.anchor));
  }
}

#[test]
fn fix_use_var_in_children() {
  let _w = fn_widget! {
    let p = @MockBox { size: Size::zero() };
    @ $p {
      opacity: 1.,
      // Use layout size query write of `p`
      @MockBox { opacity: $p.opacity }
    }
  };
}
