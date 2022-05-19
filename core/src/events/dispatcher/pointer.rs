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

macro_rules! bubble_pointer_event {
  ($ctx: ident, $event: ident, $Listener: ident) => {
    let target = $event.target();
    let mut last_bubble_from = target;
    $ctx.bubble_event(target, &mut $event, |listener: &mut $Listener, e| {
      e.position = e.context().map_from(e.position, last_bubble_from);
      last_bubble_from = target;
      listener.dispatch_event(e)
    });
  };
}

impl PointerDispatcher {
  pub fn cursor_move_to(&mut self, position: Point, ctx: &mut Context) {
    self.cursor_pos = position;
    self.pointer_enter_leave_dispatch(ctx);
    if let Some(mut event) = self.pointer_event_for_hit_widget(ctx) {
      bubble_pointer_event!(ctx, event, PointerMoveListener);
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
            let mut release_event = self.pointer_event_for_hit_widget(ctx)?;
            bubble_pointer_event!(ctx, release_event, PointerUpListener);

            let tap_on = self
              .pointer_down_uid
              .take()?
              .common_ancestor_of(release_event.target(), &ctx.widget_tree)?;
            let tap_pos = release_event
              .context()
              .map_to(release_event.position, tap_on);
            let mut tap_event =
              PointerEvent::from_mouse(tap_on, tap_pos, self.cursor_pos, self.mouse_button.1, ctx);

            bubble_pointer_event!(ctx, tap_event, TapListener);
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

      let mut wheel_event = WheelEvent {
        delta_x,
        delta_y,
        common: EventCommon::new(wid, ctx),
      };
      ctx.bubble_event(wid, &mut wheel_event, |wheel: &mut WheelListener, event| {
        wheel.dispatch_event(event)
      });
    }
  }

  fn bubble_mouse_down(&mut self, ctx: &mut Context, focus_mgr: &mut FocusManager) {
    let event = self.pointer_event_for_hit_widget(ctx);
    self.pointer_down_uid = event.as_ref().map(|e| e.target());
    let tree = &ctx.widget_tree;
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(tree).find(|id| {
        id.get(tree)
          .and_then(|w| w.query_first_type::<FocusListener>(QueryOrder::InnerFirst))
          .is_some()
      })
    });
    if let Some(focus_id) = nearest_focus {
      focus_mgr.focus(focus_id, ctx);
    } else {
      focus_mgr.blur(ctx);
    }
    if let Some(mut event) = event {
      bubble_pointer_event!(ctx, event, PointerDownListener);
    }
  }

  fn pointer_enter_leave_dispatch(&mut self, ctx: &mut Context) {
    let new_hit = self.hit_widget(ctx);

    let mut already_entered = vec![];

    for w in self.entered_widgets.drain(..) {
      if !w.is_dropped(&ctx.widget_tree) {
        continue;
      }
      match new_hit {
        Some((new_hit, _)) if w.ancestors_of(new_hit, &ctx.widget_tree) => already_entered.push(w),
        _ => {
          let old_pos = (w, &*ctx).map_from_global(self.cursor_pos);
          let mut event =
            PointerEvent::from_mouse(w, old_pos, self.cursor_pos, self.mouse_button.1, ctx);
          w.assert_get_mut(&mut ctx.widget_tree).query_all_type_mut(
            |pointer: &mut PointerLeaveListener| {
              pointer.dispatch_event(&mut event);
              !event.bubbling_canceled()
            },
            QueryOrder::InnerFirst,
          );
        }
      };
    }

    if let Some((hit_widget, _)) = new_hit {
      hit_widget
        .ancestors(&ctx.widget_tree)
        .filter(|w| {
          w.get(&ctx.widget_tree)
            .and_then(|w| w.query_first_type::<PointerEnterListener>(QueryOrder::OutsideFirst))
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
          let mut event =
            PointerEvent::from_mouse(w, old_pos, self.cursor_pos, self.mouse_button.1, ctx);

          w.assert_get_mut(&mut ctx.widget_tree).query_all_type_mut(
            |pointer: &mut PointerEnterListener| {
              pointer.dispatch_event(&mut event);
              !event.bubbling_canceled()
            },
            QueryOrder::OutsideFirst,
          );
        });
    }
  }

  fn hit_widget(&self, ctx: &Context) -> Option<(WidgetId, Point)> {
    let c_rid = ctx.widget_tree.root();
    let mut current = (c_rid, ctx).box_rect().and_then(|rect| {
      rect
        .contains(self.cursor_pos)
        .then(|| (ctx.widget_tree.root(), self.cursor_pos))
    });
    let mut hit = None;
    while let Some((id, pos)) = current {
      hit = current;
      current = id.reverse_children(&ctx.widget_tree).find_map(|c| {
        let w_ctx = (c, ctx);
        w_ctx
          .box_rect()
          // check if contain the position
          .filter(|rect| rect.contains(pos))
          .map(|_| (c_rid, w_ctx.map_from(pos, id)))
      });
    }
    hit
  }

  fn pointer_event_for_hit_widget(&self, ctx: &Context) -> Option<PointerEvent> {
    self.hit_widget(ctx).map(|(target, pos)| {
      PointerEvent::from_mouse(target, pos, self.cursor_pos, self.mouse_button.1, ctx)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::layout::{CrossAxisAlign, Row};
  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

  fn record_pointer(event_stack: Rc<RefCell<Vec<PointerEvent>>>, widget: Widget) -> Widget {
    let handler_ctor = || {
      let stack = event_stack.clone();
      move |e: &mut PointerEvent| stack.borrow_mut().push(e.clone())
    };
    widget! {
      declare Void {
        on_pointer_down : handler_ctor(),
        on_pointer_move: handler_ctor(),
        on_pointer_up: handler_ctor(),
        on_pointer_cancel: handler_ctor(),
        ExprWidget { widget }
      }
    }
  }

  #[test]
  fn mouse_pointer_bubble() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(
      event_record.clone(),
      widget! { declare Text { text: "pointer event test" }},
    );
    let root = record_pointer(
      event_record.clone(),
      widget! { declare Row { ExprWidget  { record } } },
    );
    let mut wnd = Window::without_render(root.into_widget(), Size::new(100., 100.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1., 1.).into(),
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
      widget! { declare Text { text: "pointer event test" }},
    );
    let mut wnd = Window::without_render(root, Size::new(100., 100.));
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
      widget! { declare Text { text: "pointer event test"}},
    );
    let mut wnd = Window::without_render(root, Size::new(100., 100.));
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
    #[derive(Default)]
    struct EventRecord(Rc<RefCell<Vec<PointerEvent>>>);
    impl Compose for EventRecord {
      fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> Widget {
        widget! {
          track { this }
          declare SizedBox {
            size: SizedBox::expanded_size(),
            on_pointer_down: move |e| this.0.borrow_mut().push(e.clone()),
            Text {
              text: "pointer event test",
              style: TextStyle::default(),
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

    let mut wnd = Window::without_render(root.into_widget(), Size::new(100., 100.));
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
    #[derive(Default)]
    struct EnterLeave {
      enter: Rc<RefCell<Vec<i32>>>,
      leave: Rc<RefCell<Vec<i32>>>,
    }

    impl Compose for EnterLeave {
      fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> Widget {
        widget! {
          track { this }
          declare SizedBox {
            size: SizedBox::expanded_size(),
            on_pointer_enter: move |_| this.enter.borrow_mut().push(2),
            on_pointer_leave: move |_| this.leave.borrow_mut().push(2),
            SizedBox {
              margin: EdgeInsets::all(4.),
              size: SizedBox::expanded_size(),
              on_pointer_enter: move |_| this.enter.borrow_mut().push(1),
              on_pointer_leave: move |_| this.leave.borrow_mut().push(1)

            }
          }
        }
      }
    }

    let w = EnterLeave::default();
    let enter_event = w.enter.clone();
    let leave_event = w.leave.clone();

    let mut wnd = Window::without_render(w.into_widget(), Size::new(100., 100.));
    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);

    // leave to parent
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (99, 99).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*leave_event.borrow(), &[1]);

    // leave all
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (999, 999).into(),
      modifiers: ModifiersState::default(),
    });

    assert_eq!(&*leave_event.borrow(), &[1, 2]);

    // leave event trigger by window left.
    leave_event.borrow_mut().clear();
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id });
    assert_eq!(&*leave_event.borrow(), &[1, 2]);
  }

  #[test]
  fn click() {
    #[derive(Default)]
    struct ClickPath(Rc<RefCell<i32>>);

    impl Compose for ClickPath {
      fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> Widget {
        widget! {
          track { this }
          declare Row {
            v_align: CrossAxisAlign::Start,
            on_tap: move |_| {
              let mut res = this.0.borrow_mut();
              *res += 1;
            },
            SizedBox {
              size: Size::new(100., 100.),
              on_tap: move |_| {
                let mut res = this.0.borrow_mut();
                *res += 1;
              }
            }
            SizedBox {
              size: Size::new(100., 400.)
            }
          }
        }
      }
    }

    let cp = ClickPath::default();
    let click_path = cp.0.clone();

    // Stretch row
    let mut wnd = Window::without_render(cp.into_widget(), Size::new(400., 400.));
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
    let w = widget! {
      declare Row {
        SizedBox {
          size: Size::new(50., 50.),
          tab_index: 0
        }
        SizedBox {
          size: Size::new(50., 50.)
        }
      }
    };
    let mut wnd = Window::without_render(w, Size::new(100., 100.));
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

  #[test]
  fn fix_hit_out_window() {
    let w = SizedBox { size: SizedBox::expanded_size() };
    let mut wnd = Window::without_render(w.into_widget(), Size::new(100., 100.));
    wnd.render_ready();
    wnd.dispatcher.pointer.cursor_pos = Point::new(-1., -1.);
    let hit = wnd.dispatcher.pointer.hit_widget(&wnd.context());

    assert_eq!(hit, None);
  }
}
