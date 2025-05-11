use std::cell::RefCell;

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

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
  pressed_button: PressedButtonInfo,
  /// The current global position (relative to window) of mouse
  cursor_pos: Point,
  /// The current state of the keyboard modifiers
  modifiers: ModifiersState,
}

#[derive(Default)]
struct PressedButtonInfo {
  buttons: MouseButtons,
  device_id: Option<Box<dyn DeviceId>>,
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
    if let Some(id) = wnd.focusing() {
      let e = DelayEvent::KeyBoard { key, state, physical_key, is_repeat, location, id };
      wnd.add_delay_event(e);
    } else if key == VirtualKey::Named(NamedKey::Tab) && state == ElementState::Pressed {
      wnd.add_delay_event(DelayEvent::TabFocusMove);
    }
  }

  pub fn dispatch_ime_pre_edit(&mut self, pre_edit: ImePreEdit) {
    let wnd = self.window();
    if let Some(focus_id) = wnd.focusing() {
      wnd.add_delay_event(DelayEvent::ImePreEdit { wid: focus_id, pre_edit });
    }
  }

  pub fn dispatch_receive_chars(&mut self, chars: CowArc<str>) {
    let wnd = self.window();
    if let Some(focus) = wnd.focusing() {
      self
        .window()
        .add_delay_event(DelayEvent::Chars { id: focus, chars });
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
      let new_hit = self.hit_widget();
      self.pointer_enter_leave_dispatch(new_hit);
      if let Some(hit) = new_hit {
        self
          .window()
          .add_delay_event(DelayEvent::PointerMove(hit));
      }
    }
  }

  pub fn on_cursor_left(&mut self) {
    if self.grab_mouse_wid.borrow().is_none() {
      self.info.cursor_pos = Point::new(-1., -1.);
      self.pointer_enter_leave_dispatch(self.hit_widget());
    }
  }

  pub fn dispatch_press_mouse(&mut self, device_id: Box<dyn DeviceId>, button: MouseButtons) {
    self.info.set_device_id(device_id);
    *self.info.mouse_buttons_mut() |= button;

    let hit = self.hit_widget();
    let wnd = self.window();
    let tree = wnd.tree();
    let nearest_focus = hit.and_then(|wid| {
      wid.ancestors(tree).find(|id| {
        id.query_all_iter::<MixBuiltin>(tree)
          .any(|m| m.contain_flag(MixFlags::Focus))
      })
    });
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    if let Some(focus_id) = nearest_focus {
      focus_mgr.focus(focus_id, FocusReason::Pointer);
    } else {
      focus_mgr.blur(FocusReason::Pointer);
    }

    let grab_pointer = *self.grab_mouse_wid.borrow();
    if let Some(grab_pointer) = grab_pointer {
      self
        .window()
        .add_delay_event(DelayEvent::GrabPointerDown(grab_pointer));
    } else if let Some(hit) = hit {
      self.pointer_down_wid = Some(hit);
      self
        .window()
        .add_delay_event(DelayEvent::PointerDown(hit));
    }
  }

  pub fn dispatch_release_mouse(&mut self, device_id: Box<dyn DeviceId>, button: MouseButtons) {
    self.info.set_device_id(device_id);

    let hit = self.hit_widget();

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
    self.info.mouse_buttons_mut().remove(button);
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

  fn pointer_enter_leave_dispatch(&mut self, new_hit: Option<WidgetId>) {
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
    fn deepest_test(ctx: &mut HitTestCtx, pos: &mut Point) -> Option<WidgetId> {
      // Safety: The widget tree remains read-only throughout the entire hit testing
      // process.
      let tree = unsafe { &*(ctx.tree() as *const WidgetTree) };
      let mut hit_target = None;
      loop {
        let id = ctx.id();
        let r = id.assert_get(tree);
        let HitTest { hit, can_hit_child } = r.hit_test(ctx, *pos);

        if hit {
          hit_target = Some(id);
        }

        if hit || can_hit_child {
          if let Some(c) = id.last_child(tree) {
            *pos = ctx.map_from_parent(*pos);
            ctx.set_id(c);
            continue;
          }
        }

        break;
      }

      hit_target
    }

    let mut ctx = HitTestCtx::new(self.window().tree);
    let mut pos = self.info.cursor_pos;
    let mut hit_target = deepest_test(&mut ctx, &mut pos);

    let (ctx, tree) = ctx.split_tree();
    while hit_target.is_some() && Some(ctx.id()) != hit_target {
      ctx.finish();
      let id = ctx.id();
      if let Some(sibling) = id.previous_sibling(tree) {
        ctx.set_id(sibling);
        if let Some(hit) = deepest_test(ctx, &mut pos) {
          hit_target = Some(hit);
        }
      } else if let Some(p) = id.parent(tree) {
        ctx.finish();
        ctx.set_id(p);
        pos = ctx.map_to_parent(pos);
      } else {
        break;
      }
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
  pub fn mouse_buttons(&self) -> MouseButtons { self.pressed_button.buttons }

  fn mouse_buttons_mut(&mut self) -> &mut MouseButtons { &mut self.pressed_button.buttons }

  fn is_different_device(&self, device_id: &dyn DeviceId) -> bool {
    self
      .pressed_button
      .device_id
      .as_ref()
      .is_some_and(|d| !d.is_same_device(device_id))
  }

  fn set_device_id(&mut self, device_id: Box<dyn DeviceId>) {
    if self.is_different_device(device_id.as_ref()) {
      self.pressed_button.buttons = MouseButtons::empty();
    }
    self.pressed_button.device_id = Some(device_id);
  }
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
        on_tap: move |e| $events.write().push(Info::new(e)),
        on_pointer_up: move |e| $events.write().push(Info::new(e)),
        on_pointer_cancel: move |e| $events.write().push(Info::new(e)),
      }
    };
    (w.r_into(), e2)
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

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (1., 1.).into(),
    });
    wnd.run_frame_tasks();

    assert_eq!(records.read().len(), 2);
    assert_eq!(records.read()[0].btns.bits().count_ones(), 0);
    records.write().clear();

    wnd.run_frame_tasks();

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
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
    let device_id = Box::new(DummyDeviceId);
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    wnd.process_mouse_press(device_id.clone(), MouseButtons::PRIMARY);
    wnd.run_frame_tasks();

    wnd.process_mouse_press(device_id.clone(), MouseButtons::SECONDARY);
    wnd.run_frame_tasks();

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (1, 1).into(),
    });
    wnd.run_frame_tasks();

    wnd.process_mouse_release(device_id.clone(), MouseButtons::PRIMARY);
    wnd.run_frame_tasks();

    wnd.process_mouse_release(device_id, MouseButtons::SECONDARY);
    wnd.run_frame_tasks();
    let records = records.read();
    assert_eq!(records.len(), 6);

    assert_eq!(records[0].btns, MouseButtons::PRIMARY);
    assert_eq!(records[1].btns, MouseButtons::PRIMARY | MouseButtons::SECONDARY);
    assert_eq!(records[2].btns, MouseButtons::PRIMARY | MouseButtons::SECONDARY);
    assert_eq!(records[3].btns, MouseButtons::SECONDARY);
    assert_eq!(records[4].btns, MouseButtons::SECONDARY);
    assert_eq!(records[5].btns, MouseButtons::default());
  }

  #[test]
  fn different_device_mouse() {
    reset_test_env!();

    #[derive(Clone, Copy)]
    struct WinitDeviceId(winit::event::DeviceId);

    impl DeviceId for WinitDeviceId {
      fn as_any(&self) -> &dyn std::any::Any { self }
      fn is_same_device(&self, other: &dyn DeviceId) -> bool {
        other
          .as_any()
          .downcast_ref::<WinitDeviceId>()
          .is_some_and(|other| self.0 == other.0)
      }
      fn clone_boxed(&self) -> Box<dyn DeviceId> { Box::new(WinitDeviceId(self.0)) }
    }

    let (root, record) = record_pointer();
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = Box::new(DummyDeviceId);
    let device_id_2 = WinitDeviceId(winit::event::DeviceId::dummy());

    wnd.process_mouse_press(device_id.clone(), MouseButtons::PRIMARY);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 1);

    wnd.process_mouse_press(Box::new(device_id_2), MouseButtons::SECONDARY);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 2);
    // different device clear before buttons
    assert_eq!(record.read()[1].btns, MouseButtons::SECONDARY);

    wnd.process_mouse_release(Box::new(device_id_2), MouseButtons::SECONDARY);
    wnd.run_frame_tasks();
    // a tap event is processed
    assert_eq!(record.read().len(), 4);
    assert_eq!(record.read()[3].btns, MouseButtons::empty());

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id_2.0,
      position: (1, 1).into(),
    });
    wnd.run_frame_tasks();
    // but cursor move processed.
    assert_eq!(record.read().len(), 5);
    assert_eq!(record.read()[4].btns, MouseButtons::empty());

    wnd.process_mouse_release(device_id, MouseButtons::PRIMARY);
    wnd.run_frame_tasks();
    assert_eq!(record.read().len(), 6);
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

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
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

    let device_id = winit::event::DeviceId::dummy();

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

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (50f64, 50f64).into(),
    });
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
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

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (50f64, 50f64).into(),
    });
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);

    wnd.run_frame_tasks();
    {
      let mut clicked = click_path.write();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (50f64, 50f64).into(),
    });

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (50f64, 150f64).into(),
    });
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
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

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (45f64, 45f64).into(),
    });

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);

    // point down on a focus widget
    assert!(wnd.focus_mgr.borrow().focusing().is_some());

    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: winit::event::DeviceId::dummy(),
      position: (80f64, 80f64).into(),
    });

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);

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

  #[test]
  fn fix_over_container_hit() {
    reset_test_env!();

    let mut wnd = TestWindow::new_with_size(
      mock_stack! {
        @MockBox {
          anchor: Anchor::left_top(100., 100.),
          size: Size::new(100., 100.),
         }
      },
      Size::new(500., 500.),
    );
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(105., 105.);
    let w = dispatcher.hit_widget();

    assert_ne!(w.unwrap(), wnd.tree().root());
  }
}
