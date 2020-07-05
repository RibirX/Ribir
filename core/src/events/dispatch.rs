use super::pointers::{MouseButtons, PointerEvent, PointerListener};
use crate::{
  prelude::{Point, Size},
  render::render_tree::{RenderId, RenderTree},
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::{
  cell::{Ref, RefCell},
  rc::Rc,
};
use winit::event::{DeviceId, ElementState, ModifiersState, WindowEvent};

pub(crate) struct Dispatcher {
  render_tree: Rc<RefCell<RenderTree>>,
  widget_tree: Rc<RefCell<WidgetTree>>,
  cursor_pos: Point,
  mouse_button: (Option<DeviceId>, MouseButtons),
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn new(render_tree: Rc<RefCell<RenderTree>>, widget_tree: Rc<RefCell<WidgetTree>>) -> Self {
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
          w.dispatch_pointer_move(event)
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
                  w.dispatch_pointer_down(event)
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
                  w.dispatch_pointer_up(event)
                });
              }
            }
          };
        }
      }
      _ => log::info!("not processed event {:?}", event),
    }
  }

  fn bubble_mouse_pointer<D: Fn(&PointerListener, &PointerEvent)>(&mut self, dispatch: D) {
    let mut pointer = None;
    let w_tree = self.widget_tree.borrow();

    self.hit_widget_iter().all(|(wid, pos)| {
      wid.get(&w_tree).map_or(false, |w| {
        w.downcast_ref::<PointerListener>().map_or(false, |w| {
          let event = pointer.get_or_insert_with(|| {
            PointerEvent::from_mouse(
              wid,
              pos,
              self.cursor_pos,
              self.modifiers,
              self.mouse_button.1,
            )
          });
          if event.as_ref().cancel_bubble.get() {
            false
          } else {
            event.position = pos;
            let common = event.as_mut();
            common.current_target = wid;
            common.composed_path.push(wid);
            dispatch(w, &event);
            true
          }
        })
      })
    });
  }

  /// return a iterator of widgets war hit from leaf to root.
  fn hit_widget_iter(&self) -> HitWidgetIter {
    HitWidgetIter::new(self.widget_tree.borrow(), self.render_hit_path())
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

    let r_tree = self.render_tree.borrow();
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
}

struct HitWidgetIter<'a> {
  w_tree: Ref<'a, WidgetTree>,
  render_path: Vec<(WidgetId, Point)>,
  switch: Option<(WidgetId, Point)>,
  current: Option<(WidgetId, Point)>,
}

impl<'a> HitWidgetIter<'a> {
  fn new(w_tree: Ref<'a, WidgetTree>, mut render_path: Vec<(WidgetId, Point)>) -> Self {
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
  use crate::prelude::*;
  use crate::widget::window::HeadlessWindow;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::MouseButton;

  fn record_pointer<W: Into<Box<dyn Widget>>>(
    event_stack: Rc<RefCell<Vec<PointerEvent>>>,
    child: W,
  ) -> PointerListener {
    PointerListener::listen_on(child)
      .on_pointer_down({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_move({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_up({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_cancel({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
  }

  #[test]
  fn mouse_pointer_bubble() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let root = record_pointer(event_record.clone(), record);
    let mut wnd = HeadlessWindow::headless(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
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
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
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
    let mut wnd = HeadlessWindow::headless(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
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

  #[test]
  fn different_mouse_() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let mut wnd = HeadlessWindow::headless(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    // second device press event skipped.
    assert_eq!(event_record.borrow().len(), 1);

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    // but cursor move processed.
    assert_eq!(event_record.borrow().len(), 2);
    // todo: A mouse press/release emit during another mouse's press will
    // ignored. Use difference device id to simulate it.
    // assert_eq!(event_record.borrow()[1].buttons, MouseButtons::PRIMARY);
  }

  #[test]
  fn cancel_bubble() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let pointer = PointerListener::listen_on(Text("pointer event test".to_string()))
      .on_pointer_move({
        let stack = event_record.clone();
        move |e: &PointerEvent| {
          stack.borrow_mut().push(e.clone());
          e.stop_bubbling();
        }
      });
    let root = PointerListener::listen_on(pointer).on_pointer_down({
      let stack = event_record.clone();
      move |e| stack.borrow_mut().push(e.clone())
    });
    let mut wnd = HeadlessWindow::headless(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);
  }
}
