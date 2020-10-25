use super::{CommonDispatcher, FocusManager};
use crate::{prelude::*, render::render_tree::RenderTree, widget::widget_tree::WidgetTree};
use rxrust::prelude::*;
use winit::event::{DeviceId, ElementState, MouseButton};

#[derive(Default)]
pub(crate) struct PointerDispatcher {
  cursor_pos: Point,
  last_pointer_widget: Option<WidgetId>,
  mouse_button: (Option<DeviceId>, MouseButtons),
  pointer_down_uid: Option<WidgetId>,
}

impl PointerDispatcher {
  pub fn cursor_move_to(&mut self, position: Point, common: &CommonDispatcher) {
    self.cursor_pos = position;
    self.pointer_enter_leave_dispatch(common);
    if let Some(from) = self.hit_widget(common) {
      self.bubble_pointer_from(PointerEventType::Move, common, from);
    }
  }

  pub fn on_cursor_left(&mut self, common: &CommonDispatcher) {
    self.cursor_pos = Point::new(-1., -1.);
    self.pointer_enter_leave_dispatch(common);
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    common: &CommonDispatcher,
    focus_mgr: &mut FocusManager,
  ) -> Option<()> {
    // A mouse press/release emit during another mouse's press will ignored.
    if self.mouse_button.0.get_or_insert(device_id) == &device_id {
      match state {
        ElementState::Pressed => {
          self.mouse_button.1 |= button.into();
          // only the first button press emit event.
          if self.mouse_button.1 == button.into() {
            self.bubble_mouse_down(common, focus_mgr);
          }
        }
        ElementState::Released => {
          self.mouse_button.1.remove(button.into());
          // only the last button release emit event.
          if self.mouse_button.1.is_empty() {
            self.mouse_button.0 = None;
            let release = self.hit_widget(common)?;
            self.bubble_pointer_from(PointerEventType::Up, common, release);

            let (release_on, release_pos) = release;

            let tap_on = self
              .pointer_down_uid
              .take()?
              .common_ancestor_of(release_on, common.widget_tree_ref())?;
            let tap_pos = release_on.map_to(
              release_pos,
              tap_on,
              common.widget_tree_ref(),
              common.render_tree_ref(),
            );

            self.bubble_pointer_from(PointerEventType::Tap, common, (tap_on, tap_pos));
          }
        }
      };
    }
    Some(())
  }

  fn bubble_mouse_down(&mut self, common: &CommonDispatcher, focus_mgr: &mut FocusManager) {
    let tree = common.widget_tree_ref();
    let hit = self.hit_widget(common);
    self.pointer_down_uid = hit.map(|(wid, _)| wid);
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid
        .ancestors(tree)
        .find(|id| id.get(tree).map_or(false, |w| w.has_attr::<FocusAttr>()))
    });
    if let Some(focus_id) = nearest_focus {
      focus_mgr.focus(focus_id, common);
    } else {
      focus_mgr.blur(common);
    }
    if let Some(from) = hit {
      self.bubble_pointer_from(PointerEventType::Down, common, from);
    }
  }

  fn bubble_pointer_from(
    &self,
    event_type: PointerEventType,
    common: &CommonDispatcher,
    from: (WidgetId, Point),
  ) {
    let (wid, pos) = from;
    let event = self.mouse_pointer(wid, pos, common);
    common.bubble_dispatch(
      wid,
      Self::event_emitter(event_type),
      event,
      Self::event_position_updater(wid, common),
    );
  }

  fn pointer_enter_leave_dispatch(&mut self, common: &CommonDispatcher) {
    let tree = common.widget_tree_ref();
    let new_hit = self.hit_widget(common);
    let mut old_path = self
      .last_pointer_widget
      .map(|wid| wid.ancestors(tree).collect::<Vec<_>>());
    let mut new_path = new_hit.map(|(wid, _)| wid.ancestors(tree).collect::<Vec<_>>());

    // remove the common ancestor
    if let Some(ref mut old_path) = old_path {
      if let Some(ref mut new_path) = new_path {
        while !old_path.is_empty() && old_path.last() == new_path.last() {
          old_path.pop();
          new_path.pop();
        }
      }
    }

    if let Some(old_path) = old_path {
      if let Some(old_on) = old_path.first() {
        let old_pos = old_on.map_from_global(self.cursor_pos, tree, common.render_tree_ref());
        let event = self.mouse_pointer(*old_on, old_pos, common);
        let mut pos_update = Self::event_position_updater(*old_on, common);
        let _ = old_path.iter().try_fold(event, |mut event, wid| {
          event.as_mut().current_target = *wid;
          pos_update(&mut event);
          event = common.dispatch_to(
            *wid,
            &mut Self::event_emitter(PointerEventType::Leave),
            event,
          );
          CommonDispatcher::ok_bubble(event)
        });
      }
    }

    if let Some(new_path) = new_path {
      if let Some(enter_from) = new_path.last() {
        let pos = enter_from.map_from_global(self.cursor_pos, tree, common.render_tree_ref());
        let event = self.mouse_pointer(*enter_from, pos, common);
        let mut last_enter = *enter_from;
        let _ = new_path.iter().rev().try_fold(event, |mut event, wid| {
          event.as_mut().current_target = *wid;
          event.position = wid.map_from(pos, last_enter, tree, common.render_tree_ref());
          last_enter = *event.target();

          event = common.dispatch_to(
            *wid,
            &mut Self::event_emitter(PointerEventType::Enter),
            event,
          );
          CommonDispatcher::ok_bubble(event)
        });
      }
    }

    self.last_pointer_widget = new_hit.map(|(wid, _)| wid);
  }

  fn mouse_pointer(&self, target: WidgetId, pos: Point, common: &CommonDispatcher) -> PointerEvent {
    PointerEvent::from_mouse(
      target,
      pos,
      self.cursor_pos,
      common.modifiers,
      self.mouse_button.1,
      common.window.clone(),
    )
  }

  fn hit_widget(&self, common: &CommonDispatcher) -> Option<(WidgetId, Point)> {
    fn down_coordinate_to(
      id: RenderId,
      pos: Point,
      tree: &RenderTree,
    ) -> Option<(RenderId, Point)> {
      id.layout_box_rect(tree)
        // check if contain the position
        .filter(|rect| rect.contains(pos))
        .map(|_| (id, id.map_from_parent(pos, tree)))
    }

    let r_tree = common.render_tree_ref();
    let mut current = r_tree
      .root()
      .and_then(|id| down_coordinate_to(id, self.cursor_pos, &r_tree));
    let mut hit = None;

    while let Some((rid, pos)) = current {
      if current.is_some() {
        hit = current;
      }
      current = rid
        .reverse_children(&r_tree)
        .find_map(|rid| down_coordinate_to(rid, pos, &r_tree));
    }

    hit.and_then(|(rid, pos)| {
      rid
        .relative_to_widget(common.render_tree_ref())
        .map(|wid| (wid, pos))
    })
  }

  fn event_position_updater<'r>(
    init_from: WidgetId,
    common: &'r CommonDispatcher,
  ) -> impl FnMut(&mut PointerEvent) + 'r {
    let mut last_bubble_from = init_from;
    move |e: &mut PointerEvent| {
      e.position = last_bubble_from.map_to(
        e.position,
        *e.target(),
        common.widget_tree_ref(),
        common.render_tree_ref(),
      );
      last_bubble_from = init_from;
    }
  }
  fn event_emitter(
    event_type: PointerEventType,
  ) -> impl FnMut(&PointerListener<BoxWidget>, std::rc::Rc<PointerEvent>) {
    move |listener: &PointerListener<BoxWidget>, event: std::rc::Rc<PointerEvent>| {
      log::info!("{:?} {:?}", event_type, event);
      listener.pointer_observable().next((event_type, event));
    }
  }
}

impl WidgetId {
  fn map_to(self, pos: Point, ancestor: WidgetId, tree: &WidgetTree, r_tree: &RenderTree) -> Point {
    let rid = self.relative_to_render(tree).expect("must have");
    let map_to = ancestor.relative_to_render(tree).expect("must have");

    rid.map_to(pos, map_to, r_tree)
  }

  fn map_from(
    self,
    pos: Point,
    ancestor: WidgetId,
    tree: &WidgetTree,
    r_tree: &RenderTree,
  ) -> Point {
    let rid = self.relative_to_render(tree).expect("must have");
    let map_from = ancestor.relative_to_render(tree).expect("must have");

    rid.map_from(pos, map_from, r_tree)
  }

  fn map_from_global(self, pos: Point, tree: &WidgetTree, r_tree: &RenderTree) -> Point {
    let rid = self.relative_to_render(tree).expect("must have");
    rid.map_from_global(pos, r_tree)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::{
    layout::{CrossAxisAlign, Row},
    window::NoRenderWindow,
  };
  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

  fn record_pointer<W: AttributeAttach>(
    event_stack: Rc<RefCell<Vec<PointerEvent>>>,
    widget: W,
  ) -> PointerListener<W::HostWidget> {
    let handler_ctor = || {
      let stack = event_stack.clone();
      move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
    };
    widget
      .on_pointer_down(handler_ctor())
      .on_pointer_move(handler_ctor())
      .on_pointer_up(handler_ctor())
      .on_pointer_cancel(handler_ctor())
  }

  #[test]
  fn mouse_pointer_bubble() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let root = record_pointer(event_record.clone(), Row::default().push(record));
    let mut wnd = NoRenderWindow::without_render(root, Size::new(100., 100.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records.len(), 2);
      assert_eq!(records[0].button_num(), 0);
      records.clear();
    }

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    let mut records = event_record.borrow_mut();
    assert_eq!(records[0].button_num(), 1);
    assert_eq!(records[0].position, (1., 1.).into());
    records.clear();
  }

  #[test]
  fn mouse_buttons() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let mut wnd = NoRenderWindow::without_render(root, Size::new(100., 100.));
    wnd.render_ready();

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

    assert_eq!(records[0].buttons, MouseButtons::PRIMARY);
    assert_eq!(
      records[1].buttons,
      MouseButtons::PRIMARY | MouseButtons::SECONDARY
    );
    assert_eq!(records[2].buttons, MouseButtons::default());
  }

  // Can not mock two different device id for macos.
  #[cfg(not(target_os = "macos"))]
  #[test]
  fn different_device_mouse() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let mut wnd = NoRenderWindow::without_render(root, Size::new(100., 100.));
    wnd.render_ready();

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
      (&mut id as *mut DeviceId).write_bytes(1, std::mem::size_of::<DeviceId>());
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
    assert_eq!(event_record.borrow()[1].buttons, MouseButtons::PRIMARY);

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
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = SizedBox::expanded(Text("pointer event test".to_string()).on_pointer_down({
      let stack = event_record.clone();
      move |e| {
        stack.borrow_mut().push(e.clone());
        e.stop_bubbling();
      }
    }))
    .on_pointer_down({
      let stack = event_record.clone();
      move |e| stack.borrow_mut().push(e.clone())
    });

    let mut wnd = NoRenderWindow::without_render(root.box_it(), Size::new(100., 100.));
    wnd.render_ready();

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
    let enter_event = Rc::new(RefCell::new(vec![]));
    let leave_event = Rc::new(RefCell::new(vec![]));

    let c_enter_event = enter_event.clone();
    let c_leave_event = leave_event.clone();
    let child = SizedBox::empty_box(Size::new(f32::INFINITY, f32::INFINITY))
      .on_pointer_enter(move |_| c_enter_event.borrow_mut().push(1))
      .on_pointer_leave(move |_| c_leave_event.borrow_mut().push(1));
    let c_enter_event = enter_event.clone();
    let c_leave_event = leave_event.clone();
    let parent = SizedBox::expanded(child)
      .on_pointer_enter(move |_| c_enter_event.borrow_mut().push(2))
      .on_pointer_leave(move |_| c_leave_event.borrow_mut().push(2));

    let mut wnd = NoRenderWindow::without_render(parent, Size::new(100., 100.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1000, 1000).into(),
      modifiers: ModifiersState::default(),
    });

    assert_eq!(&*leave_event.borrow(), &[1, 2]);

    // leave event trigger by window left.
    leave_event.borrow_mut().clear();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id });
    assert_eq!(&*leave_event.borrow(), &[1, 2]);
  }

  #[test]
  fn click() {
    let click_path = Rc::new(RefCell::new(0));
    let c_click_path = click_path.clone();
    let child = SizedBox::empty_box(Size::new(100., 100.)).on_tap(move |_| {
      let mut res = c_click_path.borrow_mut();
      *res += 1;
    });

    let c_click_path = click_path.clone();
    let parent = Row::default()
      .with_cross_align(CrossAxisAlign::Start)
      .push(child)
      // Stretch row
      .push(SizedBox::empty_box(Size::new(100., 400.)))
      .on_tap(move |_| {
        let mut res = c_click_path.borrow_mut();
        *res += 1;
      });
    let mut wnd = NoRenderWindow::without_render(parent, Size::new(400., 400.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50, 50).into(),
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
      let mut clicked = click_path.borrow_mut();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50, 50).into(),
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
      position: (50, 150).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });

    {
      let clicked = click_path.borrow_mut();
      assert_eq!(*clicked, 1);
    }
  }

  #[test]
  fn focus_change_by_event() {
    let root = Row::default()
      .push(SizedBox::empty_box(Size::new(50., 50.)).with_tab_index(0))
      .push(SizedBox::empty_box(Size::new(50., 50.)));
    let mut wnd = NoRenderWindow::without_render(root.box_it(), Size::new(100., 100.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (45, 45).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    // point down on a focus widget
    assert!(wnd.dispatcher.focus_mgr.focusing().is_some());

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (80, 80).into(),
      modifiers,
    });
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    assert!(wnd.dispatcher.focus_mgr.focusing().is_none());
  }
}
