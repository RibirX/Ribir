use crate::{prelude::*, render::render_tree::RenderTree, widget::widget_tree::WidgetTree};
mod focus_mgr;
pub(crate) use focus_mgr::FocusManager;
mod pointer;
pub(crate) use pointer::PointerDispatcher;
mod common;
pub(crate) use common::CommonDispatcher;
use std::{cell::RefCell, ptr::NonNull, rc::Rc};
pub use window::RawWindow;
use winit::event::WindowEvent;

pub(crate) struct Dispatcher {
  pub(crate) common: CommonDispatcher,
  pub(crate) pointer: PointerDispatcher,
  pub(crate) focus_mgr: FocusManager,
}

impl Dispatcher {
  pub fn new(
    render_tree: NonNull<RenderTree>,
    widget_tree: NonNull<WidgetTree>,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Self {
    Self {
      common: CommonDispatcher::new(render_tree, widget_tree, window),
      pointer: PointerDispatcher::default(),
      focus_mgr: FocusManager::default(),
    }
  }

  pub fn dispatch(&mut self, event: WindowEvent) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.common.modifiers_change(s),
      WindowEvent::CursorMoved { position, .. } => self.pointer.cursor_move_to(
        Point::new(position.x as f32, position.y as f32),
        &self.common,
      ),
      WindowEvent::CursorLeft { .. } => self.pointer.on_cursor_left(&self.common),
      WindowEvent::MouseInput {
        state,
        button,
        device_id,
        ..
      } => self.pointer.dispatch_mouse_input(
        device_id,
        state,
        button,
        &self.common,
        &mut self.focus_mgr,
      ),
      _ => log::info!("not processed event {:?}", event),
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
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

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
