use crate::{prelude::*, widget_tree::WidgetTree, window::CursorIcon};

pub trait DispatchInfo {
  fn modifiers(&self) -> ModifiersState;
  fn set_modifiers(&mut self, modifiers: ModifiersState);
  fn set_cursor_icon(&mut self, icon: CursorIcon);
  fn cursor_icon_mut(&mut self) -> &mut Option<CursorIcon>;
  fn stage_cursor_icon(&self) -> Option<CursorIcon>;
  fn global_pos(&self) -> Point;
  fn cursor_pos(&self) -> Point;
  fn set_cursor_pos(&mut self, pos: Point);
  fn mouse_button_device_id(&self) -> &Option<Box<dyn DeviceId>>;
  fn set_mouse_button_device_id(&mut self, device_id: Option<Box<dyn DeviceId>>);
  fn or_insert_mouse_button_device_id(
    &mut self,
    device_id: Box<dyn DeviceId>,
  ) ;
  fn mouse_button(&self) -> MouseButtons;
  fn set_mouse_button(&mut self, buttons: MouseButtons);
  fn remove_mouse_button(&mut self, buttons: MouseButtons);
}

impl WidgetTree {
  pub fn bubble_event<Ty>(&mut self, event: &mut Ty::Event)
  where
    Ty: EventListener + 'static,
  {
    self.bubble_event_with(event, |listener: &Ty, event| listener.dispatch(event));
  }

  pub fn bubble_event_with<Ty, D, E>(&self, event: &mut E, mut dispatcher: D)
  where
    D: FnMut(&Ty, &mut E),
    E: std::borrow::BorrowMut<EventCommon>,
    Ty: 'static,
  {
    loop {
      let current_target = event.borrow().current_target;
      current_target.assert_get(&self.arena).query_all_type(
        |listener: &Ty| {
          dispatcher(listener, event);
          !event.borrow_mut().bubbling_canceled()
        },
        QueryOrder::InnerFirst,
      );

      if event.borrow().bubbling_canceled() {
        break;
      }

      if let Some(p) = current_target.parent(&self.arena) {
        event.borrow_mut().current_target = p;
      } else {
        break;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

  struct Info {
    pos: Point,
    btns: MouseButtons,
  }

  fn record_pointer(event_stack: Rc<RefCell<Vec<Info>>>, widget: Widget) -> Widget {
    let handler_ctor = move || {
      let stack = event_stack.clone();

      move |e: &mut PointerEvent| {
        stack.borrow_mut().push(Info {
          pos: e.position(),
          btns: e.mouse_buttons(),
        })
      }
    };
    widget! {
      DynWidget {
        dyns: widget,
        on_pointer_down : handler_ctor(),
        on_pointer_move: handler_ctor(),
        on_pointer_up: handler_ctor(),
        on_pointer_cancel: handler_ctor(),
      }
    }
  }

  #[test]
  fn mouse_pointer_bubble() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } },
    );
    let root = record_pointer(
      event_record.clone(),
      widget! { MockMulti { DynWidget  { dyns: record } } },
    );
    let mut wnd = Window::default_mock(root, None);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1., 1.).into(),
      modifiers: ModifiersState::default(),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records.len(), 2);
      assert_eq!(records[0].btns.bits().count_ones(), 0);
      records.clear();
    }

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    let mut records = event_record.borrow_mut();
    assert_eq!(records[0].btns.bits().count_ones(), 1);
    assert_eq!(records[0].pos, (1., 1.).into());
    records.clear();
  }

  #[test]
  fn mouse_buttons() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } },
    );
    let mut wnd = Window::default_mock(root, None);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    let records = event_record.borrow();
    assert_eq!(records.len(), 3);

    assert_eq!(records[0].btns, MouseButtons::PRIMARY);
    assert_eq!(
      records[1].btns,
      MouseButtons::PRIMARY | MouseButtons::SECONDARY
    );
    assert_eq!(records[2].btns, MouseButtons::default());
  }

  // Can not mock two different device id for macos.
  #[cfg(not(target_os = "macos"))]
  #[test]
  fn different_device_mouse() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } },
    );
    let mut wnd = Window::default_mock(root, None);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);

    // A mouse press/release emit during another mouse's press will be ignored.
    let device_id_2 = unsafe {
      let mut id = DeviceId::dummy();
      (&mut id as *mut DeviceId).write_bytes(1, 1);
      id
    };
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });
    assert_eq!(event_record.borrow().len(), 1);

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id_2,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    // but cursor move processed.
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow()[1].btns, MouseButtons::PRIMARY);

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 3);
  }

  #[test]
  fn cancel_bubble() {
    #[derive(Default)]
    struct EventRecord(Rc<RefCell<Vec<PointerEvent>>>);
    impl Compose for EventRecord {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: INFINITY_SIZE,
            on_pointer_down: move |e| { this.0.borrow_mut().push(e.clone()); },

            MockBox {
              size: Size::new(100., 30.),
              on_pointer_down: move |e| {
                this.0.borrow_mut().push(e.clone());
                e.stop_bubbling();
              }
            }
          }
        }
      }
    }

    let root = EventRecord::default();
    let event_record = root.0.clone();

    let mut wnd = Window::default_mock(root.into_widget(), Some(Size::new(100., 100.)));
    wnd.draw_frame();

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { DeviceId::dummy() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);
  }

  #[test]
  fn enter_leave() {
    #[derive(Default)]
    struct EnterLeave {
      enter: Rc<RefCell<Vec<i32>>>,
      leave: Rc<RefCell<Vec<i32>>>,
    }

    impl Compose for EnterLeave {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: INFINITY_SIZE,
            on_pointer_enter: move |_| { this.enter.borrow_mut().push(2); },
            on_pointer_leave: move |_| { this.leave.borrow_mut().push(2); },
            MockBox {
              margin: EdgeInsets::all(4.),
              size: INFINITY_SIZE,
              on_pointer_enter: move |_| { this.enter.borrow_mut().push(1); },
              on_pointer_leave: move |_| { this.leave.borrow_mut().push(1); }
            }
          }
        }
      }
    }

    let w = EnterLeave::default();
    let enter_event = w.enter.clone();
    let leave_event = w.leave.clone();

    let mut wnd = Window::default_mock(w.into_widget(), Some(Size::new(100., 100.)));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);

    // leave to parent
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (99, 99).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*leave_event.borrow(), &[1]);

    // move in same widget,
    // check if duplicate event fired.
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (99, 99).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);
    assert_eq!(&*leave_event.borrow(), &[1]);

    // leave all
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (999, 999).into(),
      modifiers: ModifiersState::default(),
    });

    assert_eq!(&*leave_event.borrow(), &[1, 2]);

    // leave event trigger by window left.
    leave_event.borrow_mut().clear();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id });
    assert_eq!(&*leave_event.borrow(), &[1, 2]);
  }

  #[test]
  fn click() {
    let click_path = Stateful::new(0);
    let w = widget! {
      states { click_path: click_path.clone() }
      MockMulti {
        on_tap: move |_| *click_path += 1,
        MockBox {
          size: Size::new(100., 100.),
          on_tap: move |_| *click_path += 1,
        }
        MockBox { size: Size::new(100., 400.) }
      }
    };

    // Stretch row
    let mut wnd = Window::default_mock(w, Some(Size::new(400., 400.)));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
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

    {
      let mut clicked = click_path.state_ref();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 150f64).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });

    {
      let clicked = click_path.state_ref();
      assert_eq!(*clicked, 1);
    }
  }

  #[test]
  fn focus_change_by_event() {
    let w = widget! {
      MockMulti {
        MockBox {
          size: Size::new(50., 50.),
          tab_index: 0
        }
        MockBox {
          size: Size::new(50., 50.)
        }
      }
    };
    let mut wnd = Window::default_mock(w, Some(Size::new(100., 100.)));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (45f64, 45f64).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    // point down on a focus widget
    assert!(wnd.dispatcher.focusing().is_some());

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (80f64, 80f64).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    assert!(wnd.dispatcher.focusing().is_none());
  }

  #[test]
  fn fix_hit_out_window() {
    let w = MockBox { size: INFINITY_SIZE };
    let mut wnd = Window::default_mock(w.into_widget(), None);
    wnd.draw_frame();
    wnd.dispatcher.info.cursor_pos = Point::new(-1., -1.);
    let hit = wnd.dispatcher.hit_widget(&wnd.widget_tree);

    assert_eq!(hit, None);
  }
}
