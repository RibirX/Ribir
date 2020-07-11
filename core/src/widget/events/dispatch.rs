use super::{
  pointers::{MouseButtons, PointerEvent, PointerEventType, PointerListener},
  EventCommon,
};
use crate::{
  prelude::*,
  render::render_tree::{RenderId, RenderTree},
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::ptr::NonNull;
use winit::event::{DeviceId, ElementState, ModifiersState, WindowEvent};

pub(crate) struct Dispatcher {
  render_tree: NonNull<RenderTree>,
  widget_tree: NonNull<WidgetTree>,
  cursor_pos: Point,
  mouse_button: (Option<DeviceId>, MouseButtons),
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn new(render_tree: NonNull<RenderTree>, widget_tree: NonNull<WidgetTree>) -> Self {
    Self {
      render_tree,
      widget_tree,
      cursor_pos: Point::zero(),
      modifiers: <_>::default(),
      mouse_button: <_>::default(),
    }
  }

  pub fn dispatch(&mut self, event: WindowEvent) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = Point::new(position.x as f32, position.y as f32);
        self.bubble_mouse_pointer(|w, event| {
          log::info!("Pointer move {:?}", event);
          w.dispatch(PointerEventType::Move, event)
        });
      }
      WindowEvent::MouseInput {
        state,
        button,
        device_id,
        ..
      } => {
        // A mouse press/release emit during another mouse's press will ignored.
        if self.mouse_button.0.get_or_insert(device_id) == &device_id {
          match state {
            ElementState::Pressed => {
              self.mouse_button.1 |= button.into();
              // only the first button press emit event.
              if self.mouse_button.1 == button.into() {
                self.bubble_mouse_pointer(|w, event| {
                  log::info!("Pointer down {:?}", event);
                  w.dispatch(PointerEventType::Down, event)
                });
              }
            }
            ElementState::Released => {
              self.mouse_button.1.remove(button.into());
              // only the last button release emit event.
              if self.mouse_button.1.is_empty() {
                self.mouse_button.0 = None;
                self.bubble_mouse_pointer(|w, event| {
                  log::info!("Pointer up {:?}", event);
                  event.pressure = 0.;
                  w.dispatch(PointerEventType::Up, event)
                });
              }
            }
          };
        }
      }
      _ => log::info!("not processed event {:?}", event),
    }
  }

  fn bubble_mouse_pointer<D: FnMut(&mut PointerListener, &mut PointerEvent)>(
    &mut self,
    mut dispatch: D,
  ) {
    let mut pointer = None;

    let mut w_tree = self.widget_tree;
    self.hit_widget_iter().all(|(wid, pos)| {
      let event = pointer.get_or_insert_with(|| {
        PointerEvent::from_mouse(
          wid,
          pos,
          self.cursor_pos,
          self.modifiers,
          self.mouse_button.1,
        )
      });
      event.position = pos;
      Self::dispatch_to_widget(wid, unsafe { w_tree.as_mut() }, &mut dispatch, event);
      !event.as_mut().cancel_bubble.get()
    });
  }

  fn dispatch_to_widget<
    T: Widget,
    E: std::convert::AsMut<EventCommon>,
    H: FnMut(&mut T, &mut E),
  >(
    wid: WidgetId,
    tree: &mut WidgetTree,
    handler: &mut H,
    event: &mut E,
  ) {
    let event_widget = wid
      .get_mut(tree)
      .and_then(|w| Widget::dynamic_cast_mut::<T>(w));
    if let Some(w) = event_widget {
      let common = event.as_mut();
      common.current_target = wid;
      common.composed_path.push(wid);
      handler(w, event);
    }
  }

  /// return a iterator of widgets war hit from leaf to root.
  fn hit_widget_iter(&self) -> HitWidgetIter {
    HitWidgetIter::new(unsafe { self.widget_tree.as_ref() }, self.render_hit_path())
  }

  /// collect the render widget hit path.
  fn render_hit_path(&self) -> Vec<(WidgetId, Point)> {
    fn down_coordinate_to(
      id: RenderId,
      pos: Point,
      tree: &RenderTree,
    ) -> Option<(RenderId, Point)> {
      id.box_place(tree)
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
}

struct HitWidgetIter<'a> {
  w_tree: &'a WidgetTree,
  render_path: Vec<(WidgetId, Point)>,
  switch: Option<(WidgetId, Point)>,
  current: Option<(WidgetId, Point)>,
}

impl<'a> HitWidgetIter<'a> {
  fn new(w_tree: &'a WidgetTree, mut render_path: Vec<(WidgetId, Point)>) -> Self {
    let current = render_path.pop();
    let switch = render_path.pop();
    Self {
      w_tree,
      render_path,
      current,
      switch,
    }
  }
}

impl<'a> Iterator for HitWidgetIter<'a> {
  type Item = (WidgetId, Point);
  fn next(&mut self) -> Option<Self::Item> {
    let next = self.current.and_then(|(wid, pos)| {
      wid.parent(&self.w_tree).map(|p| {
        let pos = self
          .switch
          .filter(|(switch_wid, _)| *switch_wid == p)
          .map_or(pos, |(_, switch_pos)| {
            self.switch = self.render_path.pop();
            switch_pos
          });
        (p, pos)
      })
    });
    std::mem::replace(&mut self.current, next)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::window::NoRenderWindow;
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
    let root = record_pointer(event_record.clone(), RowColumn::row(vec![record]));
    let mut wnd = NoRenderWindow::without_render(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    let device_id = mock_device_id(0);
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
    let mut wnd = NoRenderWindow::without_render(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    let device_id = mock_device_id(0);
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
    let mut wnd = NoRenderWindow::without_render(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    let device_id = mock_device_id(0);
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);

    // A mouse press/release emit during another mouse's press will be ignored.
    let device_id_2 = mock_device_id(1);
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

    let mut wnd = NoRenderWindow::without_render(root.box_it(), DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: mock_device_id(0),
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);
  }

  fn mock_device_id(value: u8) -> DeviceId {
    unsafe {
      let mut id = std::mem::MaybeUninit::<DeviceId>::uninit();
      id.as_mut_ptr()
        .write_bytes(value, std::mem::size_of::<DeviceId>());
      id.assume_init()
    }
  }
}
