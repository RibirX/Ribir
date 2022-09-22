use std::{cell::Cell, rc::Rc};

use ribir::{
  prelude::*,
  test::{root_and_children_rect, widget_and_its_children_box_rect},
};

use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

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
     SizedBox {
      id: size2,
      size,
    }
   }
  };

  let flex_size = Size::new(1000., 500.);
  let mut wnd = Window::without_render(w, Size::new(2000., 2000.));
  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, flex_size);

  tap_at(&mut wnd, (1, 1));

  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, flex_size * 2.);
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

  let mut wnd = Window::without_render(w.into_widget(), Size::new(400., 400.));
  wnd.draw_frame();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, BEFORE_SIZE.into());
  assert_eq!(child_rect[0], BEFORE_SIZE.into());

  tap_at(&mut wnd, (25, 25));

  wnd.draw_frame();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, AFTER_TAP_SIZE.into());
  assert_eq!(child_rect[0], AFTER_TAP_SIZE.into());
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

  let mut wnd = Window::without_render(w, Size::new(2000., 2000.));
  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);

  assert_eq!(rect.size, Size::new(104., 52.));

  tap_at(&mut wnd, (60, 1));

  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(70., 60.));
}

#[test]
fn expression_for_children() {
  let embed_expr = widget! {
    Flex {
      on_tap: move |_| sized_box.size = Size::new(5., 5.),
      SizedBox { id: sized_box, size: Size::new(1., 1.) }
      // todo: how should we hint user, he/she need wrap inner widget of `ExprWidget` to track named widget change.
      ExprWidget { expr: (0..3).map(move |_| widget!{ SizedBox { size: sized_box.size } }) }
      ExprWidget {
         expr: (sized_box.size.area() > 2.).then(|| widget!{ SizedBox { size: sized_box.size } })
      }
    }
  };

  let mut wnd = Window::without_render(embed_expr, Size::new(2000., 2000.));
  wnd.draw_frame();
  let (rect, children) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(4., 1.)));
  assert_eq!(children.len(), 5);

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();

  let (rect, children) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(25., 5.)));
  assert_eq!(children.len(), 5);
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
      ExprWidget {
        expr: (0..3).map(move |_| widget!{ SizedBox { size: first.size } } )
      }
    }
  };

  let mut wnd = Window::without_render(w, Size::new(2000., 2000.));
  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(4., 1.)));

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();

  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(8., 2.)));
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
    on a.size + b.size ~> c.size
  };
  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.draw_frame();
  let (rect, _) = root_and_children_rect(&mut wnd);
  // data flow not affect on init.
  assert_eq!(rect.size, Size::new(3., 1.));

  tap_at(&mut wnd, (0, 0));
  wnd.draw_frame();

  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(8., 4.));
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
  let (rect, child_rect) = widget_and_its_children_box_rect(w, Size::new(500., 500.));
  assert_eq!(rect.size, EXPECT_SIZE * 2.);
  assert_eq!(child_rect[0].size, EXPECT_SIZE * 2.);
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

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
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
      #[skip_nc]
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

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.draw_frame();
  tap_at(&mut wnd, (1, 1));
  wnd.draw_frame();
  assert_eq!(icon_track.get(), CursorIcon::Help);
}

fn tap_at(wnd: &mut Window, pos: (i32, i32)) {
  let device_id = unsafe { DeviceId::dummy() };
  let modifiers = ModifiersState::default();

  wnd.processes_native_event(WindowEvent::CursorMoved {
    device_id,
    position: pos.into(),
    modifiers,
  });
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id,
    state: ElementState::Pressed,
    button: MouseButton::Left,
    modifiers,
  });
  wnd.processes_native_event(WindowEvent::MouseInput {
    device_id,
    state: ElementState::Released,
    button: MouseButton::Left,
    modifiers,
  });
}

#[test]
fn fix_builtin_field_can_declare_as_widget() {
  let w = widget! {
    Margin {
      margin: EdgeInsets::all(1.),
      Void {}
    }
  };

  let wnd = Window::without_render(w, Size::zero());
  assert_eq!(wnd.widget_count(), 2);
}
