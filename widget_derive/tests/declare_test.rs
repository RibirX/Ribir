#![feature(trivial_bounds, negative_impls)]

use ribir::{
  prelude::*,
  test::{root_and_children_rect, widget_and_its_children_box_rect},
};
use window::NoRenderWindow;
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

#[test]
fn declare_builder_smoke() {
  #[derive(Declare)]
  struct A;

  #[derive(Declare, Default)]
  struct B {
    a: f32,
    b: i32,
  }

  let _: A = ABuilder {}.build();
  let b = BBuilder { b: 1, ..<_>::default() }.build();
  let default_b = BBuilder { ..Default::default() }.build();
  assert_eq!(b.b, 1);
  assert_eq!(default_b.b, 0);
}

#[test]
fn declare_smoke() {
  let w = declare! {
     SizedBox {
       size: Size::new(500.,500.),
       background: Color::RED,
     }
  };

  assert!(matches!(w, BoxedWidget::SingleChild(_)));
}

#[test]
fn simple_ref_bind_work() {
  let size = Size::new(500., 500.);
  let w = declare! {
   Flex {
     ..<_>::default(),
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
  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, flex_size);

  tap_at(&mut wnd, (1, 1));

  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, flex_size * 2.);
}

#[test]
fn event_attr_sugar_work() {
  let before_size = Size::new(50., 50.);
  let after_tap_size = Size::new(100., 100.);
  let w = declare! {
    SizedBox {
      id: size_box,
      size: before_size,
      SizedBox {
        size: size_box.size,
        on_tap: move |_| size_box.size = after_tap_size,
      }
    }
  };

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.render_ready();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, before_size.into());
  assert_eq!(child_rect[0], before_size.into());

  tap_at(&mut wnd, (25, 25));

  wnd.render_ready();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, after_tap_size.into());
  assert_eq!(child_rect[0], after_tap_size.into());
}

#[test]
fn widget_wrap_bind_work() {
  let size = Size::new(50., 50.);
  let w = declare! {
    Flex {
      ..<_>::default(),
      SizedBox {
        id: sibling,
        margin: EdgeInsets::all(1.0),
        size,
      }
      SizedBox {
        margin: sibling.margin.clone(),
        size: if sibling.margin.left > 1. { Size::zero() } else { sibling.size },
        on_tap: move |_| sibling.margin = EdgeInsets::all(5.),
      }
    }
  };

  let mut wnd = Window::without_render(w, Size::new(2000., 2000.));
  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);

  assert_eq!(rect.size, Size::new(104., 52.));

  tap_at(&mut wnd, (60, 1));

  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(70., 60.));
}

#[test]
fn expression_for_children() {
  #[stateful]
  struct EmbedExpr(Size);

  impl CombinationWidget for EmbedExpr {
    fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
      let size = self.0;
      declare! {
        Flex {
          ..<_>::default(),
          SizedBox { size }
          (0..3).map(|_| declare!{
            SizedBox {
              size,
            }
          }),
          (self.0.area() > 2.).then(|| SizedBox { size } )
        }
      }
    }
  }

  let w = EmbedExpr(Size::new(1., 1.)).into_stateful();
  let mut state_ref = w.state_ref();
  let w = w.on_tap(move |_| state_ref.0 = Size::new(5., 5.));

  let mut wnd = Window::without_render(w.box_it(), Size::new(2000., 2000.));
  wnd.render_ready();
  let (rect, children) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(4., 1.)));
  assert_eq!(children.len(), 4);

  tap_at(&mut wnd, (0, 0));
  wnd.render_ready();

  let (rect, children) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(25., 5.)));
  assert_eq!(children.len(), 5);
}

#[test]
fn embed_declare_ref_outside() {
  let size = Size::new(1., 1.);
  let w = declare! {
    Flex {
      ..<_>::default(),
      SizedBox {
        id: first,
        size,
        on_tap: move |_| first.size = Size::new(2., 2.)
      }
      (0..3).map(|_| declare!{
        SizedBox {
          size: first.size,
        }
      })
    }
  };

  let mut wnd = Window::without_render(w.box_it(), Size::new(2000., 2000.));
  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(4., 1.)));

  tap_at(&mut wnd, (0, 0));
  wnd.render_ready();

  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, Rect::new(Point::zero(), Size::new(8., 2.)));
}

#[test]
fn data_flow_macro() {
  let size = Size::new(1., 1.);
  let w = declare! {
    Flex {
      on_tap: move |_| a.size *= 2.,
      ..<_>::default(),
      SizedBox{
        id: c,
        size,
      }
      SizedBox {
        id: a,
        size,
      }
      SizedBox{
        id: b,
        size: a.size
      }
    }
    data_flow!{
      a.size + b.size ~> c.size
    }
  };

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(4., 2.));

  tap_at(&mut wnd, (1, 1));
  wnd.render_ready();

  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(8., 4.));
}

#[test]
fn local_var_not_bind() {
  let expect_size = Size::new(5., 5.);
  let be_clipped_size = Size::new(500., 500.);
  let w = declare! {
    SizedBox {
      size: {
        let _size_box = expect_size;
        let _size_box_def = expect_size;
        _size_box + _size_box_def
      },
      SizedBox {
        id: _size_box,
        size: be_clipped_size,
      }
    }
  };

  let (rect, child_rect) = widget_and_its_children_box_rect(w, Size::new(500., 500.));
  assert_eq!(rect.size, expect_size * 2.);
  assert_eq!(child_rect[0].size, expect_size * 2.);
}

#[test]

fn with_attr_ref() {
  let w = declare! {
    Flex {
      cursor: tap_box.get_cursor().unwrap().clone(),
      ..<_>::default(),

      SizedBox {
        id: tap_box,
        size: Size::new(5., 5.),
        cursor: CursorIcon::Hand,
        on_tap: move |_| {
          let _ = tap_box.try_set_cursor(CursorIcon::AllScroll);
        }
      }
    }
  };

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.render_ready();

  fn root_cursor(wnd: &mut NoRenderWindow) -> Option<CursorIcon> {
    let tree = &*wnd.widget_tree();
    tree
      .root()
      .and_then(|root| root.get(tree))
      .and_then(|w| (w as &dyn AttrsAccess).get_cursor())
  }

  assert_eq!(root_cursor(&mut wnd), Some(CursorIcon::Hand));
  tap_at(&mut wnd, (1, 1));
  wnd.render_ready();
  assert_eq!(root_cursor(&mut wnd), Some(CursorIcon::AllScroll));
}

#[test]
fn attr_bind_to_self() {
  let w = declare! {
    SizedBox {
      id: self_id,
      size: Size::new(5., 5.),
      #[skip_nc]
      cursor: if self_id.size.area() < 100. {
        CursorIcon::Hand
      } else {
        CursorIcon::Help
      },
      on_tap: move |_|  self_id.size = Size::new(20.,20.),
    }
  };

  let mut wnd = Window::without_render(w, Size::new(400., 400.));
  wnd.render_ready();
  tap_at(&mut wnd, (1, 1));
  wnd.render_ready();
  let w_tree = wnd.widget_tree();
  let w = w_tree.root().and_then(|r| r.get(&*w_tree)).unwrap();
  assert_eq!(w.get_cursor(), Some(CursorIcon::Help));
}

fn tap_at(wnd: &mut NoRenderWindow, pos: (i32, i32)) {
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
