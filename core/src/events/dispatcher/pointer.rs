use super::FocusManager;
use crate::context::Context;
use crate::prelude::*;
use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta};

#[derive(Default)]
pub(crate) struct PointerDispatcher {
  cursor_pos: Point,
  entered_widgets: Vec<WidgetId>,
  mouse_button: (Option<DeviceId>, MouseButtons),
  pointer_down_uid: Option<WidgetId>,
}

impl PointerDispatcher {
  pub fn cursor_move_to(&mut self, position: Point, ctx: &mut Context) {
    self.cursor_pos = position;
    self.pointer_enter_leave_dispatch(ctx);
    if let Some(from) = self.hit_widget(ctx) {
      self.bubble_pointer_from(PointerEventType::Move, ctx, from);
    }
  }

  pub fn on_cursor_left(&mut self, ctx: &mut Context) {
    self.cursor_pos = Point::new(-1., -1.);
    self.pointer_enter_leave_dispatch(ctx);
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    ctx: &mut Context,
    focus_mgr: &mut FocusManager,
  ) -> Option<()> {
    // A mouse press/release emit during another mouse's press will ignored.
    if self.mouse_button.0.get_or_insert(device_id) == &device_id {
      match state {
        ElementState::Pressed => {
          self.mouse_button.1 |= button.into();
          // only the first button press emit event.
          if self.mouse_button.1 == button.into() {
            self.bubble_mouse_down(ctx, focus_mgr);
          }
        }
        ElementState::Released => {
          self.mouse_button.1.remove(button.into());
          // only the last button release emit event.
          if self.mouse_button.1.is_empty() {
            self.mouse_button.0 = None;
            let release = self.hit_widget(ctx)?;
            self.bubble_pointer_from(PointerEventType::Up, ctx, release);

            let (release_on, release_pos) = release;

            let tap_on = self
              .pointer_down_uid
              .take()?
              .common_ancestor_of(release_on, &ctx.widget_tree)?;
            let tap_pos = (release_on, &*ctx).map_to(release_pos, tap_on);

            self.bubble_pointer_from(PointerEventType::Tap, ctx, (tap_on, tap_pos));
          }
        }
      };
    }
    Some(())
  }

  pub fn dispatch_wheel(&mut self, delta: MouseScrollDelta, ctx: &mut Context) {
    if let Some((wid, _)) = self.hit_widget(ctx) {
      let (delta_x, delta_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x, y),
        MouseScrollDelta::PixelDelta(delta) => {
          let winit::dpi::LogicalPosition { x, y } =
            delta.to_logical(ctx.painter.device_scale() as f64);
          (x, y)
        }
      };

      ctx.bubble_event(
        wid,
        |ctx, wid| WheelEvent {
          delta_x,
          delta_y,
          common: EventCommon::new(wid, ctx),
        },
        |wheel: &WheelAttr, event| wheel.dispatch_event(event),
      );
    }
  }

  fn bubble_mouse_down(&mut self, ctx: &mut Context, focus_mgr: &mut FocusManager) {
    let tree = &ctx.widget_tree;
    let hit = self.hit_widget(ctx);
    self.pointer_down_uid = hit.map(|(wid, _)| wid);
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(tree).find(|id| {
        id.get(tree).map_or(false, |w| {
          w.as_attrs()
            .and_then(Attributes::find::<FocusAttr>)
            .is_some()
        })
      })
    });
    if let Some(focus_id) = nearest_focus {
      focus_mgr.focus(focus_id, ctx);
    } else {
      focus_mgr.blur(ctx);
    }
    if let Some(from) = hit {
      self.bubble_pointer_from(PointerEventType::Down, ctx, from);
    }
  }

  fn bubble_pointer_from(
    &self,
    event_type: PointerEventType,
    ctx: &mut Context,
    from: (WidgetId, Point),
  ) {
    let (wid, pos) = from;
    let mut last_bubble_from = wid;
    ctx.bubble_event(
      wid,
      |ctx, wid| self.mouse_pointer(wid, pos, ctx),
      |attr: &PointerAttr, e| {
        e.position = (last_bubble_from, &*ctx).map_to(e.position, e.target());
        last_bubble_from = wid;
        attr.dispatch_event(event_type, e)
      },
    );
  }

  fn pointer_enter_leave_dispatch(&mut self, ctx: &mut Context) {
    let new_hit = self.hit_widget(ctx);

    let mut already_entered = vec![];

    self.entered_widgets.iter().for_each(|w| {
      // if the widget is not the ancestor of the hit widget
      if !w.is_dropped(&ctx.widget_tree) {
        if new_hit.is_none()
          || !w
            .ancestors(&ctx.widget_tree)
            .any(|w| Some(w) == new_hit.as_ref().map(|h| h.0))
        {
          let old_pos = (*w, &*ctx).map_from_global(self.cursor_pos);
          let mut event = self.mouse_pointer(*w, old_pos, ctx);
          if let Some(pointer) = ctx.find_attr::<PointerAttr>(*w) {
            pointer.dispatch_event(PointerEventType::Leave, &mut event)
          }
        } else {
          already_entered.push(*w)
        }
      }
    });
    self.entered_widgets.clear();

    if let Some((hit_widget, _)) = new_hit {
      hit_widget
        .ancestors(&ctx.widget_tree)
        .filter(|w| {
          w.get(&ctx.widget_tree)
            .and_then(AsAttrs::find_attr::<PointerAttr>)
            .is_some()
        })
        .for_each(|w| self.entered_widgets.push(w));

      self
        .entered_widgets
        .iter()
        .rev()
        .filter(|w| !already_entered.iter().any(|e| e != *w))
        .for_each(|&w| {
          let old_pos = (w, &*ctx).map_from_global(self.cursor_pos);
          let mut event = self.mouse_pointer(w, old_pos, ctx);
          if let Some(pointer) = ctx.find_attr::<PointerAttr>(w) {
            pointer.dispatch_event(PointerEventType::Enter, &mut event);
          }
        });
    }
  }

  fn mouse_pointer(&self, target: WidgetId, pos: Point, ctx: &Context) -> PointerEvent {
    PointerEvent::from_mouse(target, pos, self.cursor_pos, self.mouse_button.1, ctx)
  }

  fn hit_widget(&self, ctx: &Context) -> Option<(WidgetId, Point)> {
    fn down_coordinate_to(id: WidgetId, pos: Point, ctx: &Context) -> Option<(WidgetId, Point)> {
      let rid = id.render_widget(&ctx.widget_tree).unwrap();
      let w_ctx = (rid, ctx);
      w_ctx
        .box_rect()
        // check if contain the position
        .filter(|rect| rect.contains(pos))
        .map(|_| (id, w_ctx.map_from(pos, id)))
    }

    let mut current = down_coordinate_to(ctx.widget_tree.root(), self.cursor_pos, ctx);
    let mut hit = None;
    while let Some((id, pos)) = current {
      hit = current;
      current = id
        .reverse_children(&ctx.widget_tree)
        .find_map(|c| down_coordinate_to(c, pos, ctx));
    }
    hit
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::layout::{CrossAxisAlign, Row};
  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

  fn record_pointer<W: AttachAttr>(
    event_stack: Rc<RefCell<Vec<PointerEvent>>>,
    widget: W,
  ) -> W::Target
  where
    W::Target: AttachAttr<Target = W::Target>,
  {
    let handler_ctor = || {
      let stack = event_stack.clone();
      move |e: &mut PointerEvent| stack.borrow_mut().push(e.clone())
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
    let record = record_pointer(
      event_record.clone(),
      Text {
        text: "pointer event test".into(),
        style: TextStyle::default(),
      },
    );
    let root = record_pointer(event_record.clone(), Row::default()).have(record.box_it());
    let mut wnd = Window::without_render(root.box_it(), Size::new(100., 100.));
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
    let root = record_pointer(
      event_record.clone(),
      Text {
        text: "pointer event test".into(),
        style: TextStyle::default(),
      },
    );
    let mut wnd = Window::without_render(root.box_it(), Size::new(100., 100.));
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
    let root = record_pointer(
      event_record.clone(),
      Text {
        text: "pointer event test".into(),
        style: TextStyle::default(),
      },
    );
    let mut wnd = Window::without_render(root.box_it(), Size::new(100., 100.));
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
    let root = declare! {
      SizedBox {
        size: SizedBox::expanded_size(),
        on_pointer_down: {
          let stack = event_record.clone();
          move |e| stack.borrow_mut().push(e.clone())
        },
        Text {
          text: "pointer event test",
          style: TextStyle::default(),
          on_pointer_down: {
            let stack = event_record.clone();
            move |e| {
              stack.borrow_mut().push(e.clone());
              e.stop_bubbling();
            }
          }
        }
      }
    };

    let mut wnd = Window::without_render(root.box_it(), Size::new(100., 100.));
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

    let parent = declare! {
      SizedBox {
        size: SizedBox::expanded_size(),
        on_pointer_enter: {
          let enter_event = enter_event.clone();
          move |_| enter_event.borrow_mut().push(2)
        },
        on_pointer_leave: {
          let leave_event = leave_event.clone();
          move |_| leave_event.borrow_mut().push(2)
        },
        SizedBox {
          size: SizedBox::expanded_size(),
          on_pointer_enter: {
            let enter_event = enter_event.clone();
            move |_| enter_event.borrow_mut().push(1)
          },
          on_pointer_leave: {
            let leave_event = leave_event.clone();
            move |_| leave_event.borrow_mut().push(1)
          }
        }
      }
    };

    let mut wnd = Window::without_render(parent.box_it(), Size::new(100., 100.));
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

    let parent = declare! {
      Row {
        cross_align: CrossAxisAlign::Start,
        on_tap: {
          let click_path = click_path.clone();
          move |_| {
          let mut res = click_path.borrow_mut();
          *res += 1;
        }},
        ..<_>::default(),
        SizedBox {
          size: Size::new(100., 100.),
          on_tap: {
            let click_path = click_path.clone();
            move |_| {
            let mut res = click_path.borrow_mut();
            *res += 1;
            }
          }
        }
        SizedBox {
          size: Size::new(100., 400.)
        }
      }
    };

    // Stretch row
    let mut wnd = Window::without_render(parent.box_it(), Size::new(400., 400.));
    wnd.render_ready();

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
      let mut clicked = click_path.borrow_mut();
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
      let clicked = click_path.borrow_mut();
      assert_eq!(*clicked, 1);
    }
  }

  #[test]
  fn focus_change_by_event() {
    let root = declare! {
      Row {
        ..<_>::default(),
        SizedBox {
          size: Size::new(50., 50.),
          tab_index: 0
        }
        SizedBox {
          size: Size::new(50., 50.)
        }
      }
    };

    let mut wnd = Window::without_render(root.box_it(), Size::new(100., 100.));
    wnd.render_ready();

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
    assert!(wnd.dispatcher.focus_mgr.focusing().is_some());

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

    assert!(wnd.dispatcher.focus_mgr.focusing().is_none());
  }
}
