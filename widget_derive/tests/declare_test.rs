use std::{cell::Cell, rc::Rc};

use ribir::{
  prelude::*,
  test::{root_and_children_rect, widget_and_its_children_box_rect},
};

use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};

#[test]
fn declare_smoke() {
  struct T;
  impl CombinationWidget for T {
    #[widget]

    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          size: Size::new(500.,500.),
          background: Color::RED,
         }
      }
    }
  }
}

#[test]
fn simple_ref_bind_work() {
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      let size = Size::new(500., 500.);
      widget! {
       declare Flex {
         SizedBox {
           size: size2.size,
           on_tap: move |_| size2.size *= 2.,
         }
         SizedBox {
          id: size2,
          size,
        }
       }
      }
    }
  }

  let flex_size = Size::new(1000., 500.);
  let mut wnd = Window::without_render(T.box_it(), Size::new(2000., 2000.));
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
  const BEFORE_SIZE: Size = Size::new(50., 50.);
  const AFTER_TAP_SIZE: Size = Size::new(100., 100.);
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          id: sized_box,
          size: BEFORE_SIZE,
          SizedBox {
            size: sized_box.size,
            on_tap: move |_| sized_box.size = AFTER_TAP_SIZE,
          }
        }
      }
    }
  }

  let mut wnd = Window::without_render(T.box_it(), Size::new(400., 400.));
  wnd.render_ready();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, BEFORE_SIZE.into());
  assert_eq!(child_rect[0], BEFORE_SIZE.into());

  tap_at(&mut wnd, (25, 25));

  wnd.render_ready();
  let (rect, child_rect) = root_and_children_rect(&mut wnd);
  assert_eq!(rect, AFTER_TAP_SIZE.into());
  assert_eq!(child_rect[0], AFTER_TAP_SIZE.into());
}

#[test]
fn widget_wrap_bind_work() {
  struct T;
  impl CombinationWidget for T {
    #[widget]

    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare Flex {
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
      }
    }
  }

  let mut wnd = Window::without_render(T.box_it(), Size::new(2000., 2000.));
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
  struct EmbedExpr(Size);

  impl CombinationWidget for EmbedExpr {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      let size = self.0;
      widget! {
        declare Flex {
          SizedBox { size }
          (0..3).map(|_| SizedBox { size}),
          (self.0.area() > 2.).then(|| SizedBox { size } )
        }
      }
    }
  }

  let w = EmbedExpr(Size::new(1., 1.)).into_stateful();
  let mut state_ref = unsafe { w.state_ref() };
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
fn embed_widget_ref_outside() {
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare Flex {
          SizedBox {
            id: first,
            size: Size::new(1., 1.),
            on_tap: move |_| first.size = Size::new(2., 2.)
          }
          // todo: should warning use id in embed expression widget without declare keyword.
          // without `declare` compile pass but unit test pass.
          (0..3).map(|_| declare SizedBox { size: first.size } )
        }
      }
    }
  }

  let mut wnd = Window::without_render(T.box_it(), Size::new(2000., 2000.));
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
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      let size = Size::new(1., 1.);
      widget! {
        declare Flex {
          on_tap: move |_| a.size *= 2.,
          SizedBox { id: c, size }
          SizedBox { id: a, size }
          SizedBox { id: b, size: a.size }
        }
        dataflows { a.size + b.size ~> c.size }
      }
    }
  }

  let mut wnd = Window::without_render(T.box_it(), Size::new(400., 400.));
  wnd.render_ready();
  let (rect, _) = root_and_children_rect(&mut wnd);
  // data flow not affect on init.
  assert_eq!(rect.size, Size::new(3., 1.));

  tap_at(&mut wnd, (0, 0));
  wnd.render_ready();

  let (rect, _) = root_and_children_rect(&mut wnd);
  assert_eq!(rect.size, Size::new(8., 4.));
}

#[test]
fn local_var_not_bind() {
  const EXPECT_SIZE: Size = Size::new(5., 5.);
  const BE_CLIPPED_SIZE: Size = Size::new(500., 500.);

  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
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
    }
  }

  let (rect, child_rect) = widget_and_its_children_box_rect(T.box_it(), Size::new(500., 500.));
  assert_eq!(rect.size, EXPECT_SIZE * 2.);
  assert_eq!(child_rect[0].size, EXPECT_SIZE * 2.);
}

#[test]

fn with_attr_ref() {
  #[derive(Default)]
  struct Track(Rc<Cell<Option<StateRef<Flex>>>>);
  impl CombinationWidget for Track {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare Flex {
          id: root,
          cursor: tap_box.get_cursor().unwrap().clone(),
          // a hack method to capture widget reference only for test, should not use
          // it in product code.
          {
            self.0.set(Some(root));
            Option::<SizedBox>::None
          }
          SizedBox {
            id: tap_box,
            size: Size::new(5., 5.),
            cursor: CursorIcon::Hand,
            on_tap: move |_| {
              let _ = tap_box.try_set_cursor(CursorIcon::AllScroll);
            }
          }
        }
      }
    }
  }
  let w = Track::default();
  let root_ref = w.0.clone();

  let mut wnd = Window::without_render(w.box_it(), Size::new(400., 400.));
  wnd.render_ready();

  assert_eq!(
    root_ref.get().and_then(|w| w.get_cursor()),
    Some(CursorIcon::Hand)
  );
  tap_at(&mut wnd, (1, 1));
  wnd.render_ready();
  assert_eq!(
    root_ref.get().and_then(|w| w.get_cursor()),
    Some(CursorIcon::AllScroll)
  );
}
#[test]
fn if_guard_field_true() {
  struct GuardTrue;
  impl CombinationWidget for GuardTrue {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          size if true => : Size::new(100., 100.)
        }
      }
    }
  }

  let (rect, _) = widget_and_its_children_box_rect(GuardTrue.box_it(), Size::new(1000., 1000.));
  assert_eq!(rect.size, Size::new(100., 100.));
}

#[test]
#[should_panic = "Required field `SizedBox::size` not set"]
fn if_guard_field_false() {
  struct GuardFalse;
  impl CombinationWidget for GuardFalse {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          size if false => : Size::new(100., 100.)
        }
      }
    }
  }
  widget_and_its_children_box_rect(GuardFalse.box_it(), Size::new(1000., 1000.));
}

#[test]
fn attr_bind_to_self() {
  #[derive(Default)]
  struct Track(Rc<Cell<Option<StateRef<SizedBox>>>>);
  impl CombinationWidget for Track {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          id: self_id,
          size: Size::new(5., 5.),
          #[skip_nc]
          cursor: if self_id.size.area() < 100. {
            CursorIcon::Hand
          } else {
            CursorIcon::Help
          },
          on_tap: move |_|  self_id.size = Size::new(20.,20.),
          // a hack method to capture widget reference only for test, should not use
          // in product code.
          {
            self.0.set(Some(self_id));
            Option::<SizedBox>::None
          }
        }
      }
    }
  }

  let w = Track::default();
  let root_ref = w.0.clone();

  let mut wnd = Window::without_render(w.box_it(), Size::new(400., 400.));
  wnd.render_ready();
  tap_at(&mut wnd, (1, 1));
  wnd.render_ready();
  assert_eq!(
    root_ref.get().and_then(|w| w.get_cursor()),
    Some(CursorIcon::Help)
  );
}

#[test]
fn if_guard_work() {
  struct T;
  impl CombinationWidget for T {
    #[widget]
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      widget! {
        declare SizedBox {
          size if true => : Size::new(100., 100.),
          margin if false =>: EdgeInsets::all(1.),
          cursor if true =>: CursorIcon::Hand
        }
      }
    }
  }

  let (rect, _) = widget_and_its_children_box_rect(T.box_it(), Size::new(500., 500.));
  assert_eq!(rect.size, Size::new(100., 100.));
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
