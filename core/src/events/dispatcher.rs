use std::cell::RefCell;

use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use crate::{
  prelude::*,
  window::{DelayEvent, WindowId},
};

/// Grabs the pointer input.
///
/// The widget corresponding to the wid will receives all pointer events
/// (on_pointer_down, on_pointer_move, and on_pointer_up) until the handle's
/// release() is called or the GrabPointer is dropped; other widgets get no
/// pointer events at all.
pub struct GrabPointer(Sc<RefCell<Option<WidgetId>>>);

impl GrabPointer {
  /// Grab the pointer input to the widget corresponding to the wid.
  ///
  /// it may return None if Some wid is already grabbed.
  #[must_use]
  pub fn grab(wid: WidgetId, wnd: &Window) -> Option<Self> {
    wnd.dispatcher.borrow().grab_pointer(wid)
  }

  /// Release the pointer input.
  pub fn release(self) {}
}

impl Drop for GrabPointer {
  fn drop(&mut self) { self.0.borrow_mut().take(); }
}

pub(crate) struct Dispatcher {
  wnd_id: WindowId,
  pub(crate) info: DispatchInfo,
  pub(crate) entered_widgets: Vec<WidgetId>,
  grab_mouse_wid: Sc<RefCell<Option<WidgetId>>>,
  pointer_down_wid: Option<WidgetId>,
}

impl Dispatcher {
  pub fn new(wnd_id: WindowId) -> Self {
    Self {
      wnd_id,
      info: <_>::default(),
      entered_widgets: vec![],
      grab_mouse_wid: Sc::new(RefCell::new(None)),
      pointer_down_wid: None,
    }
  }

  pub(crate) fn grab_pointer(&self, wid: WidgetId) -> Option<GrabPointer> {
    if self.grab_mouse_wid.borrow().is_none() {
      *self.grab_mouse_wid.borrow_mut() = Some(wid);
      Some(GrabPointer(self.grab_mouse_wid.clone()))
    } else {
      None
    }
  }

  fn window(&self) -> Sc<Window> {
    AppCtx::get_window(self.wnd_id).expect("The window of the `Dispatcher` already dropped")
  }
}
#[derive(Default)]
pub(crate) struct DispatchInfo {
  /// The current state of mouse button press state.
  mouse_button: (Option<DeviceId>, MouseButtons),
  /// The current global position (relative to window) of mouse
  cursor_pos: Point,
  /// The current state of the keyboard modifiers
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, wnd_factor: f64) {
    match event {
      WindowEvent::ModifiersChanged(s) => self.info.modifiers = s.state(),
      WindowEvent::CursorMoved { position, .. } => {
        let pos = position.to_logical::<f32>(wnd_factor);
        self.cursor_move_to(Point::new(pos.x, pos.y))
      }
      WindowEvent::CursorLeft { .. } => self.on_cursor_left(),
      WindowEvent::MouseWheel { delta, .. } => self.dispatch_wheel(delta, wnd_factor),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  pub fn dispatch_keyboard_input(
    &mut self, physical_key: PhysicalKey, key: VirtualKey, is_repeat: bool, location: KeyLocation,
    state: ElementState,
  ) {
    let wnd = self.window();
    if let Some(focus_id) = wnd.focusing() {
      let event = KeyboardEvent::new(&wnd, focus_id, physical_key, key, is_repeat, location);
      match state {
        ElementState::Pressed => wnd.add_delay_event(DelayEvent::KeyDown(event)),
        ElementState::Released => wnd.add_delay_event(DelayEvent::KeyUp(event)),
      };
    } else if key == VirtualKey::Named(NamedKey::Tab) {
      wnd.add_delay_event(DelayEvent::TabFocusMove);
    }
  }

  pub fn dispatch_ime_pre_edit(&mut self, pre_edit: ImePreEdit) {
    let wnd = self.window();
    if let Some(focus_id) = wnd.focusing() {
      wnd.add_delay_event(DelayEvent::ImePreEdit { wid: focus_id, pre_edit });
    }
  }

  pub fn dispatch_receive_chars(&mut self, chars: String) {
    let wnd = self.window();
    if let Some(focus) = wnd.focusing() {
      self
        .window()
        .add_delay_event(DelayEvent::Chars { id: focus, chars });
    }
  }

  fn cursor_press_down(&mut self, hit: Option<WidgetId>) {
    let grab_pointer = *self.grab_mouse_wid.borrow();
    if let Some(grab_pointer) = grab_pointer {
      self
        .window()
        .add_delay_event(DelayEvent::GrabPointerDown(grab_pointer));
    } else {
      self.pointer_down_wid = None;
      if let Some(hit) = hit {
        self.pointer_down_wid = Some(hit);
        self
          .window()
          .add_delay_event(DelayEvent::PointerDown(hit));
      }
    }
  }

  fn cursor_press_up(&mut self, hit: Option<WidgetId>) {
    let wnd = self.window();
    let grab_pointer = *self.grab_mouse_wid.borrow();
    if let Some(grab_pointer) = grab_pointer {
      wnd.add_delay_event(DelayEvent::GrabPointerUp(grab_pointer));
    } else {
      if let Some(hit) = hit {
        wnd.add_delay_event(DelayEvent::PointerUp(hit));
        if let Some(wid) = self.pointer_down_wid {
          if let Some(p) = wid.lowest_common_ancestor(hit, wnd.tree()) {
            wnd.add_delay_event(DelayEvent::Tap(p));
          }
        }
      }
      self.pointer_down_wid = None;
    }
  }

  pub fn cursor_move_to(&mut self, position: Point) {
    self.info.cursor_pos = position;
    let grab_pointer = *self.grab_mouse_wid.borrow();
    if let Some(grab_pointer) = grab_pointer {
      self
        .window()
        .add_delay_event(DelayEvent::GrabPointerMove(grab_pointer));
    } else {
      self.pointer_enter_leave_dispatch();
      if let Some(hit) = self.hit_widget() {
        self
          .window()
          .add_delay_event(DelayEvent::PointerMove(hit));
      }
    }
  }

  pub fn on_cursor_left(&mut self) {
    if self.grab_mouse_wid.borrow().is_none() {
      self.info.cursor_pos = Point::new(-1., -1.);
      self.pointer_enter_leave_dispatch();
    }
  }

  pub fn dispatch_mouse_input(
    &mut self, device_id: DeviceId, state: ElementState, button: MouseButton,
  ) {
    // A mouse press/release emit during another mouse's press will ignored.
    if self.info.mouse_button.0.get_or_insert(device_id) == &device_id {
      match state {
        ElementState::Pressed => {
          self.info.mouse_button.1 |= button.into();
          // only the first button press emit event.
          if self.info.mouse_button.1 == button.into() {
            self.bubble_pointer_down();
          }
        }
        ElementState::Released => {
          self.info.mouse_button.1.remove(button.into());
          // only the last button release emit event.
          if self.info.mouse_button.1.is_empty() {
            self.info.mouse_button.0 = None;
            let hit = self.hit_widget();

            self.cursor_press_up(hit);
          }
        }
      };
    }
  }

  pub fn dispatch_wheel(&mut self, delta: MouseScrollDelta, wnd_factor: f64) {
    if let Some(wid) = self.hit_widget() {
      let (delta_x, delta_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * 16., y * 16.),
        MouseScrollDelta::PixelDelta(delta) => {
          let winit::dpi::LogicalPosition { x, y } = delta.to_logical(wnd_factor);
          (x, y)
        }
      };

      self
        .window()
        .add_delay_event(DelayEvent::Wheel { id: wid, delta_x, delta_y });
    }
  }

  fn bubble_pointer_down(&mut self) {
    let hit = self.hit_widget();
    let wnd = self.window();
    let tree = wnd.tree();

    let nearest_focus = hit.and_then(|wid| {
      wid.ancestors(tree).find(|id| {
        id.query_all_iter::<MixBuiltin>(tree)
          .any(|m| m.contain_flag(MixFlags::Focus))
      })
    });
    if let Some(focus_id) = nearest_focus {
      wnd.focus_mgr.borrow_mut().focus(focus_id, tree);
    } else {
      wnd.focus_mgr.borrow_mut().blur(tree);
    }

    self.cursor_press_down(hit);
  }

  fn pointer_enter_leave_dispatch(&mut self) {
    let new_hit = self.hit_widget();
    let wnd = self.window();
    let tree = wnd.tree();

    let old = self
      .entered_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(tree))
      .copied();

    if let Some(old) = old {
      let ancestor = new_hit.and_then(|w| w.lowest_common_ancestor(old, tree));
      wnd.add_delay_event(DelayEvent::PointerLeave { bottom: old, up: ancestor });
    };

    if let Some(new) = new_hit {
      let ancestor = old.and_then(|o| o.lowest_common_ancestor(new, tree));
      wnd.add_delay_event(DelayEvent::PointerEnter { bottom: new, up: ancestor });
    }

    self.entered_widgets = new_hit.map_or(vec![], |wid| wid.ancestors(tree).collect::<Vec<_>>());
  }

  fn hit_widget(&self) -> Option<WidgetId> {
    let mut hit_target = None;
    let wnd = self.window();
    let tree = wnd.tree();

    let mut w = Some(tree.root());
    let mut pos = self.info.cursor_pos;
    while let Some(id) = w {
      let r = id.assert_get(tree);
      let ctx = HitTestCtx { id, tree: wnd.tree };
      let HitTest { hit, can_hit_child } = r.hit_test(&ctx, pos);

      pos = tree.map_from_parent(id, pos);

      if hit {
        hit_target = w;
      }

      w = id
        .last_child(tree)
        .filter(|_| can_hit_child)
        .or_else(|| {
          if hit {
            return None;
          }

          pos = tree.map_to_parent(id, pos);

          let mut node = w;
          while let Some(p) = node {
            node = p.previous_sibling(tree);
            if node.is_some() {
              break;
            } else {
              node = p.parent(tree);

              if let Some(id) = node {
                pos = tree.map_to_parent(id, pos);
                if node == hit_target {
                  node = None;
                }
              }
            }
          }
          node
        });
    }
    hit_target
  }
}

impl DispatchInfo {
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.modifiers }

  #[inline]
  pub fn global_pos(&self) -> Point { self.cursor_pos }

  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.mouse_button.1 }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  struct Info {
    pos: Point,
    btns: MouseButtons,
  }

  impl Info {
    fn new(e: &PointerEvent) -> Self { Info { pos: e.position(), btns: e.mouse_buttons() } }
  }

  fn record_pointer() -> (GenWidget, Stateful<Vec<Info>>) {
    let events = Stateful::new(vec![]);
    let e2 = events.clone_writer();

    let w = fn_widget! {
      @MockBox {
        size: Size::new(100., 30.),
        on_pointer_down : move |e| $events.write().push(Info::new(e)),
        on_pointer_move: move |e| $events.write().push(Info::new(e)),
        on_pointer_up: move |e| $events.write().push(Info::new(e)),
        on_pointer_cancel: move |e| $events.write().push(Info::new(e)),
      }
    };
    (w.into(), e2)
  }

  #[test]
  fn mouse_pointer_bubble() {
    reset_test_env!();

    let (gen, records) = record_pointer();
    let events = records.clone_writer();
    let root = fn_widget! {
      @MockMulti {
        on_pointer_down : move |e| $events.write().push(Info::new(e)),
        on_pointer_move: move |e| $events.write().push(Info::new(e)),
        on_pointer_up: move |e| $events.write().push(Info::new(e)),
        on_pointer_cancel: move |e| $events.write().push(Info::new(e)),
        @ { gen.gen_widget() }
      }
    };

    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (1., 1.).into() });
    wnd.run_frame_tasks();

    assert_eq!(records.read().len(), 2);
    assert_eq!(records.read()[0].btns.bits().count_ones(), 0);
    records.write().clear();

    wnd.run_frame_tasks();

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.run_frame_tasks();
    let mut records = records.write();
    assert_eq!(records[0].btns.bits().count_ones(), 1);
    assert_eq!(records[0].pos, (1., 1.).into());
    records.clear();
  }

  #[test]
  fn mouse_buttons() {
    reset_test_env!();

    let (root, records) = record_pointer();
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.run_frame_tasks();

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Right);
    wnd.run_frame_tasks();

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (1, 1).into() });
    wnd.run_frame_tasks();

    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.run_frame_tasks();

    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Right);
    wnd.run_frame_tasks();
    let records = records.read();
    assert_eq!(records.len(), 3);

    assert_eq!(records[0].btns, MouseButtons::PRIMARY);
    assert_eq!(records[1].btns, MouseButtons::PRIMARY | MouseButtons::SECONDARY);
    assert_eq!(records[2].btns, MouseButtons::default());
  }

  // Can not mock two different device id for macos.
  #[cfg(not(target_os = "macos"))]
  #[test]
  fn different_device_mouse() {
    reset_test_env!();

    let (root, record) = record_pointer();
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 1);

    // A mouse press/release emit during another mouse's press will be ignored.
    let device_id_2 = unsafe {
      let mut id = DeviceId::dummy();
      (&mut id as *mut DeviceId).write_bytes(1, 1);
      id
    };

    wnd.process_mouse_input(device_id_2, ElementState::Pressed, MouseButton::Left);
    wnd.process_mouse_input(device_id_2, ElementState::Released, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 1);

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id_2,
      position: (1, 1).into(),
    });
    wnd.run_frame_tasks();
    // but cursor move processed.
    assert_eq!(record.read().len(), 2);
    assert_eq!(record.read().len(), 2);
    assert_eq!(record.read()[1].btns, MouseButtons::PRIMARY);

    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 3);
  }

  #[test]
  fn cancel_bubble() {
    reset_test_env!();

    let (record, writer) = split_value(vec![]);
    let w = fn_widget! {
      @MockBox {
        size: INFINITY_SIZE,
        on_pointer_down: move |e| { $writer.write().push(e.current_target()); },

        @MockBox {
          size: Size::new(100., 30.),
          on_pointer_down: move |e| {
            $writer.write().push(e.current_target());
            e.stop_propagation();
          }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    wnd.process_mouse_input(unsafe { DeviceId::dummy() }, ElementState::Pressed, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 1);
  }

  #[test]
  fn enter_leave() {
    reset_test_env!();

    let (enter, e_writer) = split_value(vec![]);
    let (leave, l_writer) = split_value(vec![]);

    let w = fn_widget! {
      @MockBox {
        size: INFINITY_SIZE,
        padding: EdgeInsets::all(4.),
        on_pointer_enter: move |_| { $e_writer.write().push(2); },
        on_pointer_leave: move |_| { $l_writer.write().push(2); },
        @MockBox {
          size: INFINITY_SIZE,
          on_pointer_enter: move |_| { $e_writer.write().push(1); },
          on_pointer_leave: move |_| { $l_writer.write().push(1); }
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (10, 10).into() });
    wnd.run_frame_tasks();
    assert_eq!(&*enter.read(), &[2, 1]);

    // leave to parent
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (99, 99).into() });
    wnd.run_frame_tasks();
    assert_eq!(&*leave.read(), &[1]);

    // move in same widget,
    // check if duplicate event fired.
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (99, 99).into() });
    wnd.run_frame_tasks();
    assert_eq!(&*enter.read(), &[2, 1]);
    assert_eq!(&*leave.read(), &[1]);

    // leave all
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (999, 999).into() });
    wnd.run_frame_tasks();
    assert_eq!(&*leave.read(), &[1, 2]);

    // leave event trigger by window left.
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved { device_id, position: (10, 10).into() });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id });
    wnd.run_frame_tasks();
    assert_eq!(&*leave.read(), &[1, 2, 1, 2]);
  }

  #[test]
  fn capture_click() {
    reset_test_env!();

    let click_path = Stateful::new(vec![]) as Stateful<Vec<usize>>;
    let c_click_path = click_path.clone_writer();
    let w = fn_widget! {
      @MockBox {
        size: Size::new(100., 100.),
        on_tap: move |_| $c_click_path.write().push(4),
        on_tap_capture: move |_| $c_click_path.write().push(1),
        @MockBox {
          size: Size::new(100., 100.),
          on_tap: move |_| $c_click_path.write().push(3),
          on_tap_capture: move |_| $c_click_path.write().push(2),
        }
      }
    };

    // Stretch row
    let mut wnd = TestWindow::new_with_size(w, Size::new(400., 400.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
    });
    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(*click_path.read(), [1, 2, 3, 4]);
  }

  #[test]
  fn click() {
    reset_test_env!();

    let click_path = Stateful::new(0);
    let c_click_path = click_path.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        on_tap: move |_| *$c_click_path.write() += 1,
        @MockBox {
          size: Size::new(100., 100.),
          on_tap: move |_| *$c_click_path.write() += 1,
        }
        @MockBox { size: Size::new(100., 400.) }
      }
    };

    // Stretch row
    let mut wnd = TestWindow::new_with_size(w, Size::new(400., 400.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
    });
    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);

    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);

    wnd.run_frame_tasks();
    {
      let mut clicked = click_path.write();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
    });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 150f64).into(),
    });
    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    wnd.run_frame_tasks();
    assert_eq!(*click_path.read(), 1);
  }

  #[test]
  fn focus_change_by_event() {
    reset_test_env!();

    let w = fn_widget! {
      @MockMulti {
        @MockBox {
          size: Size::new(50., 50.),
          tab_index: 0i16
        }
        @MockBox {
          size: Size::new(50., 50.)
        }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (45f64, 45f64).into(),
    });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);

    // point down on a focus widget
    assert!(wnd.focus_mgr.borrow().focusing().is_some());

    wnd.process_mouse_input(device_id, ElementState::Released, MouseButton::Left);
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (80f64, 80f64).into(),
    });

    wnd.process_mouse_input(device_id, ElementState::Pressed, MouseButton::Left);

    assert!(wnd.focus_mgr.borrow().focusing().is_none());
  }

  #[test]
  fn fix_hit_out_window() {
    reset_test_env!();

    let mut wnd = TestWindow::new(fn_widget!(MockBox { size: INFINITY_SIZE }));
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(-1., -1.);
    let hit = dispatcher.hit_widget();

    assert_eq!(hit, None);
  }

  #[test]
  fn normal_mode_search() {
    reset_test_env!();
    struct T {
      pub wid1: Option<WidgetId>,
      pub wid2: Option<WidgetId>,
    }
    let (data, writer) = split_value(T { wid1: None, wid2: None });

    let w = fn_widget! {
      @MockStack {
        clamp: BoxClamp::EXPAND_BOTH,
        @MockBox {
          anchor: Point::new(50., 50.),
          on_mounted: move |ctx| {
            $writer.write().wid1 = Some(ctx.id);
          },
          size: Size::new(100., 100.),
        }
        @MockBox {
          on_mounted: move |ctx| {
            $writer.write().wid2 = Some(ctx.id);
          },
          size: Size::new(50., 150.),
          anchor: Point::new(100., 100.),
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(500., 500.));
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(125., 125.);
    let hit_2 = dispatcher.hit_widget();
    assert_eq!(hit_2, data.read().wid2);

    dispatcher.info.cursor_pos = Point::new(80., 80.);
    let hit_1 = dispatcher.hit_widget();
    assert_eq!(hit_1, data.read().wid1);
  }

  #[test]
  fn fix_align_hit_test() {
    reset_test_env!();
    let (expect_hit, w_hit) = split_value(None);
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          h_align: HAlign::Center,
          v_align: VAlign::Center,
          size: Size::new(100., 100.),
          on_mounted: move |ctx| *$w_hit.write() = Some(ctx.id),
        }
      },
      Size::new(500., 500.),
    );
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(250., 250.);
    assert!(expect_hit.read().is_some());
    assert_eq!(dispatcher.hit_widget(), *expect_hit.read());
  }

  #[test]
  fn fix_transform_hit() {
    reset_test_env!();
    let (expect_hit, w_hit) = split_value(None);
    let mut wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          anchor: Point::new(50., 50.),
          transform: Transform::rotation(Angle::degrees(45.)),
          size: Size::new(100., 100.),
          on_mounted: move |ctx| *$w_hit.write() = Some(ctx.id),
        }
      },
      Size::new(500., 500.),
    );
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(51., 51.);
    assert!(expect_hit.read().is_some());
    assert_eq!(dispatcher.hit_widget(), *expect_hit.read());
  }
}
