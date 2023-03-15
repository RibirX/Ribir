use std::{cell::RefCell, rc::Rc};

use crate::{prelude::*, widget_tree::WidgetTree};
use ribir_text::PIXELS_PER_EM;
// use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta,
// WindowEvent};

use super::focus_mgr::FocusManager;

pub(crate) struct Dispatcher {
  pub(crate) focus_mgr: Rc<RefCell<FocusManager>>,
  pub(crate) focus_widgets: Vec<WidgetId>,
  pub(crate) info: DispatchInfo,
  pub(crate) entered_widgets: Vec<WidgetId>,
  pub(crate) pointer_down_uid: Option<WidgetId>,
}

impl Dispatcher {
  pub fn new(focus_mgr: Rc<RefCell<FocusManager>>) -> Self {
    Self {
      focus_mgr,
      focus_widgets: vec![],
      info: <_>::default(),
      entered_widgets: vec![],
      pointer_down_uid: None,
    }
  }
}
#[derive(Default)]
pub(crate) struct DispatchInfo {
  /// The current state of mouse button press state.
  mouse_button: (Option<Box<dyn PointerId>>, MouseButtons),
  /// The current global position (relative to window) of mouse
  cursor_pos: Point,
  /// Cursor icon try to set to window.
  cursor_icon: Option<CursorIcon>,
  /// The current state of the keyboard modifiers
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, tree: &mut WidgetTree, wnd_factor: f64) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.info.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        let logical_pos = ScaleToLogic::new(wnd_factor as f32).transform_point(position.cast());

        self.cursor_move_to(Point::new(logical_pos.x, logical_pos.y), tree)
      }
      WindowEvent::CursorLeft { .. } => self.on_cursor_left(tree),
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        self.dispatch_mouse_input(device_id, state, button, tree);
      }
      WindowEvent::KeyboardInput { input, .. } => {
        self.dispatch_keyboard_input(input, tree);
      }
      WindowEvent::ReceivedCharacter(c) => {
        self.dispatch_received_char(c, tree);
      }
      WindowEvent::MouseWheel { delta, .. } => self.dispatch_wheel(delta, tree, wnd_factor),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  pub fn dispatch_keyboard_input(
    &mut self,
    input: /* winit::event:: */ KeyboardInput,
    tree: &mut WidgetTree,
  ) {
    if let Some(key) = input.virtual_keycode {
      let prevented = if let Some(focus) = self.focusing() {
        let mut event = KeyboardEvent {
          key,
          scan_code: input.scancode,
          common: EventCommon::new(focus, tree, &self.info),
        };
        match input.state {
          ElementState::Pressed => tree.bubble_event::<KeyDownListener>(&mut event),
          ElementState::Released => tree.bubble_event::<KeyUpListener>(&mut event),
        };

        event.common.prevent_default
      } else {
        false
      };
      if !prevented {
        self.shortcut_process(key, input.state, tree);
      }
    }
  }

  pub fn dispatch_received_char(&mut self, c: char, tree: &mut WidgetTree) {
    if let Some(focus) = self.focusing() {
      let mut char_event = CharEvent {
        char: c,
        common: EventCommon::new(focus, tree, &self.info),
      };
      tree.bubble_event::<CharListener>(&mut char_event);
    }
  }

  pub fn shortcut_process(
    &mut self,
    key: VirtualKeyCode,
    state: ElementState,
    tree: &mut WidgetTree,
  ) {
    if key == VirtualKeyCode::Tab && ElementState::Pressed == state {
      if self.info.modifiers.contains(ModifiersState::SHIFT) {
        self.prev_focus_widget(tree);
      } else {
        self.next_focus_widget(tree);
      }
    }
  }

  pub fn cursor_move_to(&mut self, position: Point, tree: &mut WidgetTree) {
    self.info.cursor_pos = position;
    self.pointer_enter_leave_dispatch(tree);
    if let Some(mut event) = self.pointer_event_for_hit_widget(tree) {
      tree.bubble_event::<PointerMoveListener>(&mut event);
    }
  }

  pub fn on_cursor_left(&mut self, tree: &mut WidgetTree) {
    self.info.cursor_pos = Point::new(-1., -1.);
    self.pointer_enter_leave_dispatch(tree);
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: Box<dyn PointerId>,
    state: ElementState,
    button: MouseButtons,
    tree: &mut WidgetTree,
  ) -> Option<()> {
    // A mouse press/release emit during another mouse's press will ignored.
    if self
      .info
      .mouse_button
      .0
      .get_or_insert(device_id.clone())
      .eq(&device_id)
    {
      match state {
        ElementState::Pressed => {
          self.info.mouse_button.1 |= button;
          // only the first button press emit event.
          if self.info.mouse_button.1 == button {
            self.bubble_mouse_down(tree);
          }
        }
        ElementState::Released => {
          self.info.mouse_button.1.remove(button);
          // only the last button release emit event.
          if self.info.mouse_button.1.is_empty() {
            self.info.mouse_button.0 = None;
            let mut release_event = self.pointer_event_for_hit_widget(tree)?;
            tree.bubble_event::<PointerUpListener>(&mut release_event);

            let tap_on = self
              .pointer_down_uid
              .take()?
              .lowest_common_ancestor(release_event.target(), &tree.arena)?;
            let mut tap_event =
              PointerEvent::from_mouse(MockPointerId::zero(), tap_on, tree, &self.info);

            tree.bubble_event::<TapListener>(&mut tap_event);
          }
        }
      };
    }
    Some(())
  }

  pub fn dispatch_wheel(
    &mut self,
    delta: MouseScrollDelta,
    tree: &mut WidgetTree,
    wnd_factor: f64,
  ) {
    if let Some(wid) = self.hit_widget(tree) {
      let (delta_x, delta_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * PIXELS_PER_EM, y * PIXELS_PER_EM),
        MouseScrollDelta::PixelDelta(delta) => {
          let logical_delta = ScaleToLogic::new(wnd_factor as f32).transform_point(delta.cast());
          (logical_delta.x, logical_delta.y)
        }
      };

      let mut wheel_event = WheelEvent {
        delta_x,
        delta_y,
        common: EventCommon::new(wid, tree, &self.info),
      };
      tree.bubble_event::<WheelListener>(&mut wheel_event);
    }
  }

  pub fn take_cursor_icon(&mut self) -> Option<CursorIcon> { self.info.cursor_icon.take() }

  fn bubble_mouse_down(&mut self, tree: &mut WidgetTree) {
    let event = self.pointer_event_for_hit_widget(tree);
    self.pointer_down_uid = event.as_ref().map(|e| e.target());
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(&tree.arena).find(|id| {
        id.get(&tree.arena)
          .map_or(false, |w| w.contain_type::<FocusNode>())
      })
    });
    if let Some(focus_id) = nearest_focus {
      self.focus(focus_id, tree);
    } else {
      self.blur(tree);
    }
    if let Some(mut event) = event {
      tree.bubble_event::<PointerDownListener>(&mut event);
    }
  }

  fn pointer_enter_leave_dispatch(&mut self, tree: &mut WidgetTree) {
    let new_hit = self.hit_widget(tree);

    let arena = &tree.arena;
    let already_entered_start = new_hit
      .and_then(|new_hit| {
        self
          .entered_widgets
          .iter()
          .position(|e| e.ancestors_of(new_hit, arena))
      })
      .unwrap_or(self.entered_widgets.len());

    let mut already_entered = vec![];
    self.entered_widgets[already_entered_start..].clone_into(&mut already_entered);

    // fire leave
    self.entered_widgets[..already_entered_start]
      .iter()
      .filter(|w| !w.is_dropped(arena))
      .for_each(|l| {
        let mut event = PointerEvent::from_mouse(MockPointerId::zero(), *l, tree, &self.info);
        l.assert_get(arena).query_all_type(
          |pointer: &PointerLeaveListener| {
            pointer.dispatch(&mut event);
            !event.bubbling_canceled()
          },
          QueryOrder::InnerFirst,
        );
      });

    let new_enter_end = self.entered_widgets.get(already_entered_start).cloned();
    self.entered_widgets.clear();

    // fire new entered
    if let Some(hit_widget) = new_hit {
      // collect new entered
      for w in hit_widget.ancestors(arena) {
        if Some(w) != new_enter_end {
          let obj = w.assert_get(arena);
          if obj.contain_type::<PointerEnterListener>()
            || obj.contain_type::<PointerLeaveListener>()
          {
            self.entered_widgets.push(w);
          }
        } else {
          break;
        }
      }

      self.entered_widgets.iter().rev().for_each(|w| {
        let obj = w.assert_get(arena);
        if obj.contain_type::<PointerEnterListener>() {
          let mut event = PointerEvent::from_mouse(MockPointerId::zero(), *w, tree, &self.info);
          obj.query_all_type(
            |pointer: &PointerEnterListener| {
              pointer.dispatch(&mut event);
              !event.bubbling_canceled()
            },
            QueryOrder::InnerFirst,
          );
        }
      });
      self.entered_widgets.extend(already_entered);
    }
  }

  fn hit_widget(&self, tree: &WidgetTree) -> Option<WidgetId> {
    fn down_coordinate(id: WidgetId, pos: Point, tree: &WidgetTree) -> Option<(WidgetId, Point)> {
      let WidgetTree { arena, store, wnd_ctx, .. } = tree;

      let r = id.assert_get(arena);
      let ctx = HitTestCtx { id, arena, store, wnd_ctx };
      let hit_test = r.hit_test(&ctx, pos);

      if hit_test.hit {
        Some((id, store.map_from_parent(id, pos, arena)))
      } else if hit_test.can_hit_child {
        let pos = store.map_from_parent(id, pos, arena);
        id.reverse_children(arena)
          .find_map(|c| down_coordinate(c, pos, tree))
      } else {
        None
      }
    }

    let mut current = down_coordinate(tree.root(), self.info.cursor_pos, tree);
    let mut hit = current;
    while let Some((id, pos)) = current {
      hit = current;
      current = id
        .reverse_children(&tree.arena)
        .find_map(|c| down_coordinate(c, pos, tree));
    }
    hit.map(|(w, _)| w)
  }

  fn pointer_event_for_hit_widget(&mut self, tree: &WidgetTree) -> Option<PointerEvent> {
    self
      .hit_widget(tree)
      .map(|target| PointerEvent::from_mouse(MockPointerId::zero(), target, tree, &self.info))
  }
}

impl DispatchInfo {
  #[inline]
  pub fn set_cursor_icon(&mut self, icon: CursorIcon) { self.cursor_icon = Some(icon) }
  /// Return the cursor icon that will submit to window.
  #[inline]
  pub fn stage_cursor_icon(&self) -> Option<CursorIcon> { self.cursor_icon }

  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.modifiers }

  #[inline]
  pub fn global_pos(&self) -> Point { self.cursor_pos }

  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.mouse_button.1 }
}

impl WidgetTree {
  pub(crate) fn bubble_event<Ty>(&mut self, event: &mut Ty::Event)
  where
    Ty: EventListener + 'static,
  {
    self.bubble_event_with(event, |listener: &Ty, event| listener.dispatch(event));
  }

  pub(crate) fn bubble_event_with<Ty, D, E>(&self, event: &mut E, mut dispatcher: D)
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
  // use winit::event::WindowEvent;
  // use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

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

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: DevicePoint::new(1, 1),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records.len(), 2);
      assert_eq!(records[0].btns.bits().count_ones(), 0);
      records.clear();
    }

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
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

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::SECONDARY,
    });

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (1, 1).into(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Released,
      button: MouseButtons::PRIMARY,
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Released,
      button: MouseButtons::SECONDARY,
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

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
    });

    assert_eq!(event_record.borrow().len(), 1);

    // A mouse press/release emit during another mouse's press will be ignored.
    let device_id_2 = MockPointerId::new(1);
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2.clone(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2.clone(),
      state: ElementState::Released,
      button: MouseButtons::PRIMARY,
    });
    assert_eq!(event_record.borrow().len(), 1);

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id_2,
      position: (1, 1).into(),
    });

    // but cursor move processed.
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow()[1].btns, MouseButtons::PRIMARY);

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Released,
      button: MouseButtons::PRIMARY,
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
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
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

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (10, 10).into(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);

    // leave to parent
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (99, 99).into(),
    });
    assert_eq!(&*leave_event.borrow(), &[1]);

    // move in same widget,
    // check if duplicate event fired.
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (99, 99).into(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);
    assert_eq!(&*leave_event.borrow(), &[1]);

    // leave all
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (999, 999).into(),
    });

    assert_eq!(&*leave_event.borrow(), &[1, 2]);

    // leave event trigger by window left.
    leave_event.borrow_mut().clear();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: (10, 10).into(),
    });
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id: MockPointerId::zero() });
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

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: DevicePoint::new(50, 50),
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

    {
      let mut clicked = click_path.state_ref();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: DevicePoint::new(50, 50),
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
    });
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: MockPointerId::zero(),
      position: DevicePoint::new(50, 150),
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: MockPointerId::zero(),
      state: ElementState::Released,
      button: MouseButtons::PRIMARY,
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

    let device_id = MockPointerId::zero();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id.clone(),
      position: DevicePoint::new(45, 45),
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id.clone(),
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
    });

    // point down on a focus widget
    assert!(wnd.dispatcher.focusing().is_some());

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id.clone(),
      state: ElementState::Released,
      button: MouseButtons::PRIMARY,
    });
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id.clone(),
      position: DevicePoint::new(80, 80),
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButtons::PRIMARY,
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
