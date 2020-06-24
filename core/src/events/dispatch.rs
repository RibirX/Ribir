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
    self.render_hit_iter().all(|(wid, pos)| {
      wid
        .get(&w_tree)
        .and_then(|w| w.as_any().downcast_ref::<PointerListener>())
        .map_or(false, |w| {
          let event = pointer.get_or_insert_with(|| {
            PointerEvent::from_mouse(
              wid,
              pos,
              self.cursor_pos,
              self.modifiers,
              self.mouse_button.1,
            )
          });
          if event.as_mut().cancel_bubble.get() {
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
    });
  }

  fn render_hit_iter<'a>(&'a self) -> impl Iterator<Item = (WidgetId, Point)> + 'a {
    let r_tree = self.render_tree.clone();
    HitRenderIter::new(self.render_tree.borrow(), self.cursor_pos).filter_map(move |(rid, pos)| {
      rid
        .relative_to_widget(&r_tree.borrow())
        .map(|wid| (wid, pos))
    })
  }
}

pub struct HitRenderIter<'a> {
  r_tree: Ref<'a, RenderTree>,
  current: Option<RenderId>,
  pos: Point,
}

impl<'a> HitRenderIter<'a> {
  fn new(r_tree: Ref<'a, RenderTree>, pos: Point) -> Self {
    let mut iter = Self {
      current: r_tree.root(),
      r_tree,
      pos,
    };
    iter.current = iter
      .current
      .filter(|rid| Self::try_down_coordinate(&iter.r_tree, *rid, &mut iter.pos));
    iter
  }

  fn try_down_coordinate(tree: &RenderTree, rid: RenderId, pos: &mut Point) -> bool {
    rid
      .box_place(tree)
      .filter(|rect| rect.contains(*pos))
      .map_or(false, |rect| {
        let offset: Size = rect.min().to_tuple().into();
        *pos -= offset;
        true
      })
  }
}

impl<'a> Iterator for HitRenderIter<'a> {
  type Item = (RenderId, Point);
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(rid) = self.current {
      let value = Some((rid, self.pos));
      let Self { r_tree, pos, .. } = self;
      self.current = rid
        .reverse_children(r_tree)
        .find(|rid| Self::try_down_coordinate(r_tree, *rid, pos));

      return value;
    } else {
    }
    None
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
