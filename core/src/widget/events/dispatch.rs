use super::{
  pointers::{MouseButtons, PointerEvent, PointerEventType, PointerListener},
  EventCommon,
};
use crate::{
  prelude::*,
  render::render_tree::{RenderId, RenderTree},
  widget::events::focus::FocusManager,
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::ptr::NonNull;
use std::{cell::RefCell, rc::Rc};
pub use window::RawWindow;
use winit::event::{DeviceId, ElementState, ModifiersState, WindowEvent};

pub(crate) struct Dispatcher {
  render_tree: NonNull<RenderTree>,
  widget_tree: NonNull<WidgetTree>,
  focus_mgr: FocusManager,
  cursor_pos: Point,
  last_pointer_widget: Option<WidgetId>,
  mouse_button: (Option<DeviceId>, MouseButtons),
  modifiers: ModifiersState,
  window: Rc<RefCell<Box<dyn RawWindow>>>,
  pointer_down_uid: Option<WidgetId>,
}

impl Dispatcher {
  pub fn new(
    render_tree: NonNull<RenderTree>,
    widget_tree: NonNull<WidgetTree>,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Self {
    Self {
      render_tree,
      widget_tree,
      focus_mgr: FocusManager::default(),
      last_pointer_widget: None,
      cursor_pos: Point::zero(),
      modifiers: <_>::default(),
      mouse_button: <_>::default(),
      window,
      pointer_down_uid: None,
    }
  }

  pub fn dispatch(&mut self, event: WindowEvent) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = Point::new(position.x as f32, position.y as f32);
        self.pointer_enter_leave_dispatch();
        self.bubble_pointer(PointerEventType::Move);
      }
      WindowEvent::CursorLeft { .. } => {
        self.cursor_pos = Point::new(-1., -1.);
        self.pointer_enter_leave_dispatch();
      }
      WindowEvent::MouseInput {
        state,
        button,
        device_id,
        ..
      } => {
        // A mouse press/release emit during another mouse's press will ignored.
        if self.mouse_button.0.get_or_insert(device_id) == &device_id {
          let path = self.widget_hit_path();
          match state {
            ElementState::Pressed => {
              self.mouse_button.1 |= button.into();
              // only the first button press emit event.
              if self.mouse_button.1 == button.into() {
                self.pointer_down_uid = path.last().map(|(id, _)| *id);
                self.bubble_pointer(PointerEventType::Down);
              }
            }
            ElementState::Released => {
              self.mouse_button.1.remove(button.into());
              // only the last button release emit event.
              if self.mouse_button.1.is_empty() {
                self.mouse_button.0 = None;
                self.bubble_pointer(PointerEventType::Up);

                let release_on = path.last().map(|(id, _)| *id);
                let common_ancestor = self.pointer_down_uid.take().and_then(|down| {
                  release_on
                    .and_then(|release| down.common_ancestor_of(release, self.widget_tree_ref()))
                });
                if let Some(from) = common_ancestor {
                  let iter = path.iter().rev().skip_while(|w| w.0 != from);
                  self.bubble_pointer_by_path(PointerEventType::Tap, iter);
                }
              }
            }
          };
        }
      }
      _ => log::info!("not processed event {:?}", event),
    }
  }

  fn bubble_pointer(&mut self, event_type: PointerEventType) -> PointerEvent {
    // change the focus widget.
    if event_type == PointerEventType::Down {
      let nearest_focus = self.pointer_down_uid.and_then(|wid| {
        wid.ancestors(self.widget_tree_ref()).find_map(|id| {
          id.get(self.widget_tree_ref())
            .and_then(|widget| Widget::dynamic_cast_ref::<Focus>(widget))
            .map(|focus| (id, focus.tab_index))
        })
      });
      if let Some((focus_id, tab_index)) = nearest_focus {
        self.focus_mgr.focus(
          focus_id,
          tab_index,
          self.modifiers,
          self.window.clone(),
          unsafe { self.widget_tree.as_mut() },
        );
      } else {
        self
          .focus_mgr
          .blur(self.modifiers, self.window.clone(), unsafe {
            self.widget_tree.as_mut()
          });
      }
    }
    self.bubble_pointer_by_path(event_type, self.widget_hit_path().iter().rev())
  }

  fn bubble_pointer_by_path<'r>(
    &mut self,
    event_type: PointerEventType,
    mut path: impl Iterator<Item = &'r (WidgetId, Point)>,
  ) -> PointerEvent {
    let event = self.mouse_pointer_without_target();
    let mut init_target = false;
    let res = path.try_fold(event, |mut event, (wid, pos)| {
      if !init_target {
        event.as_mut().target = *wid;
        init_target = true;
      }
      event.position = *pos;
      event = self.dispatch_pointer(*wid, event_type, event);
      if event.as_mut().cancel_bubble.get() {
        Err(event)
      } else {
        Ok(event)
      }
    });
    match res {
      Ok(event) => event,
      Err(event) => event,
    }
  }

  fn dispatch_pointer(
    &mut self,
    wid: WidgetId,
    pointer_type: PointerEventType,
    event: PointerEvent,
  ) -> PointerEvent {
    log::info!("{:?} {:?}", pointer_type, event);
    Self::dispatch_to_widget(
      wid,
      unsafe { self.widget_tree.as_mut() },
      &mut |widget: &mut PointerListener, e| widget.dispatch(pointer_type, e),
      event,
    )
  }

  pub(crate) fn dispatch_to_widget<
    T: Widget,
    E: std::convert::AsMut<EventCommon> + std::fmt::Debug,
    H: FnMut(&mut T, Rc<E>),
  >(
    wid: WidgetId,
    tree: &mut WidgetTree,
    handler: &mut H,
    mut event: E,
  ) -> E {
    let event_widget = wid
      .get_mut(tree)
      .and_then(|w| Widget::dynamic_cast_mut::<T>(w));
    if let Some(w) = event_widget {
      let common = event.as_mut();
      common.current_target = wid;
      common.composed_path.push(wid);

      let rc_event = Rc::new(event);
      handler(w, rc_event.clone());
      event = Rc::try_unwrap(rc_event).expect("Keep the event is dangerous and not allowed");
    }
    event
  }

  fn pointer_enter_leave_dispatch(&mut self) {
    let mut event = self.mouse_pointer_without_target();
    let mut old_path = if let Some(last) = self.last_pointer_widget {
      last.ancestors(self.widget_tree_ref()).collect::<Vec<_>>()
    } else {
      vec![]
    };
    let mut new_path = self.widget_hit_path();
    // Remove the common ancestors of `old_path` and `new_path`
    while !old_path.is_empty() && old_path.last() == new_path.first().map(|(wid, _)| wid) {
      old_path.pop();
      new_path.remove(0);
    }

    event = old_path.iter().fold(event, |mut event, wid| {
      event.position = self.widget_relative_point(*wid);
      log::info!("Pointer leave {:?}", event);
      self.dispatch_pointer(*wid, PointerEventType::Leave, event)
    });

    new_path.iter().fold(event, |mut event, (wid, pos)| {
      event.position = *pos;
      log::info!("Pointer enter {:?}", event);
      self.dispatch_pointer(*wid, PointerEventType::Enter, event)
    });
    self.last_pointer_widget = new_path.last().map(|(wid, _)| *wid);
  }

  /// collect the render widget hit path.
  fn widget_hit_path(&self) -> Vec<(WidgetId, Point)> {
    fn down_coordinate_to(
      id: RenderId,
      pos: Point,
      tree: &RenderTree,
    ) -> Option<(RenderId, Point)> {
      id.layout_box_rect(tree)
        .filter(|rect| rect.contains(pos))
        .map(|rect| {
          let offset: Size = rect.min().to_tuple().into();
          (id, pos - offset)
        })
    }

    let r_tree = self.render_tree_ref();
    let mut current = r_tree
      .root()
      .and_then(|id| down_coordinate_to(id, self.cursor_pos, &r_tree));

    let mut path = vec![];
    while let Some((rid, pos)) = current {
      path.push((
        rid
          .relative_to_widget(&r_tree)
          .expect("Render object 's owner widget is not exist."),
        pos,
      ));
      current = rid
        .reverse_children(&r_tree)
        .find_map(|rid| down_coordinate_to(rid, pos, &r_tree));
    }

    path
  }

  #[inline]
  fn render_tree_ref(&self) -> &RenderTree { unsafe { self.render_tree.as_ref() } }

  #[inline]
  fn widget_tree_ref(&self) -> &WidgetTree { unsafe { self.widget_tree.as_ref() } }

  fn widget_relative_point(&self, wid: WidgetId) -> Point {
    let r_tree = self.render_tree_ref();
    if let Some(rid) = wid.relative_to_render(self.widget_tree_ref()) {
      rid
        .ancestors(r_tree)
        .map(|r| {
          r.layout_box_rect(r_tree)
            .map_or_else(Point::zero, |rect| rect.origin)
        })
        .fold(Point::zero(), |sum, pos| sum + pos.to_vector())
    } else {
      unreachable!("");
    }
  }

  fn mouse_pointer_without_target(&self) -> PointerEvent {
    unsafe {
      PointerEvent::from_mouse_with_dummy_target(
        self.cursor_pos,
        self.modifiers,
        self.mouse_button.1,
        self.window.clone(),
      )
    }
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
  use winit::event::MouseButton;

  fn record_pointer<W: Widget>(
    event_stack: Rc<RefCell<Vec<PointerEvent>>>,
    widget: W,
  ) -> BoxWidget {
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
      assert_eq!(records[0].composed_path().len(), 1);
      assert_eq!(records[1].composed_path().len(), 2);
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
    let root = Text("pointer event test".to_string())
      .on_pointer_move({
        let stack = event_record.clone();
        move |e: &PointerEvent| {
          stack.borrow_mut().push(e.clone());
          e.stop_bubbling();
        }
      })
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
    let click_path = Rc::new(RefCell::new((vec![], 0)));
    let c_click_path = click_path.clone();
    let child = SizedBox::empty_box(Size::new(100., 100.)).on_tap(move |e| {
      let mut res = c_click_path.borrow_mut();
      res.0 = e.composed_path().to_vec();
      res.1 += 1;
    });

    let c_click_path = click_path.clone();
    let parent = Row::default()
      .with_cross_align(CrossAxisAlign::Start)
      .push(child)
      // Stretch row
      .push(SizedBox::empty_box(Size::new(100., 400.)))
      .on_tap(move |e| {
        let mut res = c_click_path.borrow_mut();
        res.0 = e.composed_path().to_vec();
        res.1 += 1;
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

    let parent_id = *click_path.borrow().0.last().unwrap();
    println!("path, {:?}", &*click_path.borrow().0);
    {
      let mut clicked = click_path.borrow_mut();
      assert_eq!(clicked.0.len(), 2);
      assert_eq!(clicked.1, 2);
      clicked.0.clear();
      clicked.1 = 0;
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
      assert_eq!(&clicked.0, &[parent_id]);
      assert_eq!(clicked.1, 1);
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
