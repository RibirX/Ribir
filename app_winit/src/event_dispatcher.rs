use std::cell::RefCell;
use std::rc::Rc;

use ribir_core::prelude::focus_mgr::{common_ancestors, FocusManager};
use ribir_core::prelude::{
  BlurListener, CharEvent, CharListener, DeviceMouseButtons, EventCommon, EventListener,
  FocusEvent, FocusInListener, FocusListener, FocusNode, FocusOutListener, HitTestCtx,
  KeyDownListener, KeyUpListener, KeyboardEvent, MouseButtons, PointerDownListener,
  PointerEnterListener, PointerEvent, PointerLeaveListener, PointerMoveListener, PointerUpListener,
  TapListener, VirtualKeyCode, WheelEvent, WheelListener,
};
use ribir_core::widget::QueryOrder;
use ribir_core::widget_tree::{WidgetId, WidgetTree};
use ribir_core::window::CursorIcon;
use ribir_painter::{Point, PIXELS_PER_EM};
use winit::event::{
  DeviceId, ElementState, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent,
};

use crate::from_device_id::RDeviceId;
use crate::from_modifiers::RModifiersState;
use crate::from_mouse::RMouseButton;
use crate::from_virtual_key_code::RVirtualKeyCode;

pub(crate) struct Dispatcher {
  pub(crate) focus_mgr: Rc<RefCell<FocusManager>>,
  pub(crate) focus_widgets: Vec<WidgetId>,
  pub(crate) info: Rc<RefCell<dyn ribir_core::events::dispatcher::DispatchInfo>>,
  pub(crate) entered_widgets: Vec<WidgetId>,
  pub(crate) pointer_down_uid: Option<WidgetId>,
}

impl Dispatcher {
  pub fn new(focus_mgr: Rc<RefCell<FocusManager>>) -> Self {
    Self {
      focus_mgr,
      focus_widgets: vec![],
      info: Rc::new(RefCell::new(DispatchInfo::default())),
      entered_widgets: vec![],
      pointer_down_uid: None,
    }
  }
}

impl Default for Dispatcher {
  fn default() -> Self {
    Self {
      focus_mgr: Default::default(),
      focus_widgets: Default::default(),
      info: Rc::new(RefCell::new(DispatchInfo::default())),
      entered_widgets: Default::default(),
      pointer_down_uid: Default::default(),
    }
  }
}

#[derive(Default)]
pub(crate) struct DispatchInfo {
  // `mouse_button` is implemented as tuple instead of a more readable struct because otherwise the
  // `Default` implementaiton would not work due to `winit:event::DeviceId`.
  /// The current state of mouse button press state.
  mouse_button: DeviceMouseButtons,
  /// The current global position (relative to window) of mouse
  cursor_pos: Point,
  /// Cursor icon try to set to window.
  cursor_icon: Option<CursorIcon>,
  /// The current state of the keyboard modifiers
  modifiers: ModifiersState,
}

impl ribir_core::events::dispatcher::DispatchInfo for DispatchInfo {
  fn modifiers(&self) -> ribir_core::prelude::ModifiersState {
    RModifiersState::from(self.modifiers).into()
  }

  fn set_modifiers(&mut self, modifiers: ribir_core::prelude::ModifiersState) {
    self.modifiers = RModifiersState::from(modifiers).into();
  }

  fn set_cursor_icon(&mut self, icon: ribir_core::window::CursorIcon) {
    self.cursor_icon = Some(icon);
  }

  fn cursor_icon_mut(&mut self) -> &mut Option<CursorIcon> { &mut self.cursor_icon }

  fn stage_cursor_icon(&self) -> Option<ribir_core::window::CursorIcon> { self.cursor_icon }

  fn global_pos(&self) -> Point { self.cursor_pos }

  fn cursor_pos(&self) -> Point { self.cursor_pos }

  fn set_cursor_pos(&mut self, pos: Point) { self.cursor_pos = pos }

  fn mouse_button_device_id(&self) -> &Option<Box<dyn ribir_core::prelude::DeviceId>> {
    &self.mouse_button.device_id
  }

  fn set_mouse_button_device_id(
    &mut self,
    device_id: Option<Box<dyn ribir_core::prelude::DeviceId>>,
  ) {
    self.mouse_button.device_id = device_id;
  }

  fn or_insert_mouse_button_device_id(
    &mut self,
    device_id: Box<dyn ribir_core::prelude::DeviceId>,
  ) {
    self.mouse_button.device_id.get_or_insert(device_id);
  }

  fn mouse_button(&self) -> MouseButtons { self.mouse_button.buttons }

  fn set_mouse_button(&mut self, buttons: MouseButtons) { self.mouse_button.buttons = buttons; }

  fn remove_mouse_button(&mut self, buttons: MouseButtons) {
    self.mouse_button.buttons.remove(buttons);
  }
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, tree: &mut WidgetTree, wnd_factor: f64) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self
        .info
        .borrow_mut()
        .set_modifiers(RModifiersState::from(s).into()),
      WindowEvent::CursorMoved { position, .. } => {
        let pos = position.to_logical::<f32>(wnd_factor);
        self.cursor_move_to(Point::new(pos.x, pos.y), tree)
      }
      WindowEvent::CursorLeft { .. } => self.on_cursor_left(tree),
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        self.dispatch_mouse_input(device_id, state, button, tree);
      }
      WindowEvent::KeyboardInput { input, .. } => {
        self.dispatch_keyboard_input(input, tree);
      }
      WindowEvent::ReceivedCharacter(c) => {
        self.dispatch_received_char(c, tree);
      }
      WindowEvent::MouseWheel { delta, .. } => self.dispatch_wheel(delta, tree, wnd_factor),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  fn dispatch_keyboard_input(&mut self, input: winit::event::KeyboardInput, tree: &mut WidgetTree) {
    if let Some(key) = input.virtual_keycode {
      let prevented = if let Some(focus) = self.focusing() {
        let mut event = KeyboardEvent {
          key: RVirtualKeyCode::from(key).into(),
          scan_code: input.scancode,
          common: EventCommon::new(focus, tree, self.info.clone()),
        };
        match input.state {
          ElementState::Pressed => tree.bubble_event::<KeyDownListener>(&mut event),
          ElementState::Released => tree.bubble_event::<KeyUpListener>(&mut event),
        };

        event.common.prevent_default
      } else {
        false
      };
      if !prevented {
        self.shortcut_process(RVirtualKeyCode::from(key).into(), input.state, tree);
      }
    }
  }

  pub fn dispatch_received_char(&mut self, c: char, tree: &mut WidgetTree) {
    if let Some(focus) = self.focusing() {
      let mut char_event = CharEvent {
        char: c,
        common: EventCommon::new(focus, tree, self.info.clone()),
      };
      tree.bubble_event::<CharListener>(&mut char_event);
    }
  }

  pub fn shortcut_process(
    &mut self,
    key: VirtualKeyCode,
    state: ElementState,
    tree: &mut WidgetTree,
  ) {
    if key == VirtualKeyCode::Tab && ElementState::Pressed == state {
      if self
        .info
        .borrow()
        .modifiers()
        .contains(RModifiersState::from(ModifiersState::SHIFT).into())
      {
        self.prev_focus_widget(tree);
      } else {
        self.next_focus_widget(tree);
      }
    }
  }

  pub fn cursor_move_to(&mut self, position: Point, tree: &mut WidgetTree) {
    self.info.borrow_mut().set_cursor_pos(position);
    self.pointer_enter_leave_dispatch(tree);
    if let Some(mut event) = self.pointer_event_for_hit_widget(tree) {
      tree.bubble_event::<PointerMoveListener>(&mut event);
    }
  }

  pub fn on_cursor_left(&mut self, tree: &mut WidgetTree) {
    self.info.borrow_mut().set_cursor_pos(Point::new(-1., -1.));
    self.pointer_enter_leave_dispatch(tree);
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    tree: &mut WidgetTree,
  ) -> Option<()> {
    self
      .info
      .borrow_mut()
      .or_insert_mouse_button_device_id(Box::new(RDeviceId::from(device_id)));

    let device_id = &RDeviceId::from(device_id);
    let curr_device_id = &RDeviceId::from(
      self
        .info
        .borrow()
        .mouse_button_device_id()
        .as_ref()
        .unwrap(),
    );

    // A mouse press/release emit during another mouse's press will ignored.
    if curr_device_id.eq(device_id) {
      match state {
        ElementState::Pressed => {
          self.info.borrow_mut().set_mouse_button(
            self.info.borrow().mouse_button() | RMouseButton::from(button).into(),
          );
          // only the first button press emit event.
          if self.info.borrow().mouse_button() == RMouseButton::from(button).into() {
            self.bubble_mouse_down(tree);
          }
        }
        ElementState::Released => {
          self
            .info
            .borrow_mut()
            .remove_mouse_button(RMouseButton::from(button).into());
          // only the last button release emit event.
          if self.info.borrow().mouse_button().is_empty() {
            self.info.borrow_mut().set_mouse_button_device_id(None);
            let mut release_event = self.pointer_event_for_hit_widget(tree)?;
            tree.bubble_event::<PointerUpListener>(&mut release_event);

            let tap_on = self
              .pointer_down_uid
              .take()?
              .lowest_common_ancestor(release_event.target(), &tree.arena)?;
            let mut tap_event = PointerEvent::from_mouse(tap_on, tree, self.info.clone());

            tree.bubble_event::<TapListener>(&mut tap_event);
          }
        }
      };
    }
    Some(())
  }

  pub fn dispatch_wheel(
    &mut self,
    delta: MouseScrollDelta,
    tree: &mut WidgetTree,
    wnd_factor: f64,
  ) {
    if let Some(wid) = self.hit_widget(tree) {
      let (delta_x, delta_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * PIXELS_PER_EM, y * PIXELS_PER_EM),
        MouseScrollDelta::PixelDelta(delta) => {
          let winit::dpi::LogicalPosition { x, y } = delta.to_logical(wnd_factor);
          (x, y)
        }
      };

      let mut wheel_event = WheelEvent {
        delta_x,
        delta_y,
        common: EventCommon::new(wid, tree, self.info.clone()),
      };
      tree.bubble_event::<WheelListener>(&mut wheel_event);
    }
  }

  pub fn take_cursor_icon(&mut self) -> Option<CursorIcon> {
    self.info.borrow_mut().cursor_icon_mut().take()
  }

  fn bubble_mouse_down(&mut self, tree: &mut WidgetTree) {
    let event = self.pointer_event_for_hit_widget(tree);
    self.pointer_down_uid = event.as_ref().map(|e| e.target());
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(&tree.arena).find(|id| {
        id.get(&tree.arena)
          .map_or(false, |w| w.contain_type::<FocusNode>())
      })
    });
    if let Some(focus_id) = nearest_focus {
      self.focus(focus_id, tree);
    } else {
      self.blur(tree);
    }
    if let Some(mut event) = event {
      tree.bubble_event::<PointerDownListener>(&mut event);
    }
  }

  fn pointer_enter_leave_dispatch(&mut self, tree: &mut WidgetTree) {
    let new_hit = self.hit_widget(tree);

    let arena = &tree.arena;
    let already_entered_start = new_hit
      .and_then(|new_hit| {
        self
          .entered_widgets
          .iter()
          .position(|e| e.ancestors_of(new_hit, arena))
      })
      .unwrap_or(self.entered_widgets.len());

    let mut already_entered = vec![];
    self.entered_widgets[already_entered_start..].clone_into(&mut already_entered);

    // fire leave
    self.entered_widgets[..already_entered_start]
      .iter()
      .filter(|w| !w.is_dropped(arena))
      .for_each(|l| {
        let mut event = PointerEvent::from_mouse(*l, tree, self.info.clone());
        l.assert_get(arena).query_all_type(
          |pointer: &PointerLeaveListener| {
            pointer.dispatch(&mut event);
            !event.bubbling_canceled()
          },
          QueryOrder::InnerFirst,
        );
      });

    let new_enter_end = self.entered_widgets.get(already_entered_start).cloned();
    self.entered_widgets.clear();

    // fire new entered
    if let Some(hit_widget) = new_hit {
      // collect new entered
      for w in hit_widget.ancestors(arena) {
        if Some(w) != new_enter_end {
          let obj = w.assert_get(arena);
          if obj.contain_type::<PointerEnterListener>()
            || obj.contain_type::<PointerLeaveListener>()
          {
            self.entered_widgets.push(w);
          }
        } else {
          break;
        }
      }

      self.entered_widgets.iter().rev().for_each(|w| {
        let obj = w.assert_get(arena);
        if obj.contain_type::<PointerEnterListener>() {
          let mut event = PointerEvent::from_mouse(*w, tree, self.info.clone());
          obj.query_all_type(
            |pointer: &PointerEnterListener| {
              pointer.dispatch(&mut event);
              !event.bubbling_canceled()
            },
            QueryOrder::InnerFirst,
          );
        }
      });
      self.entered_widgets.extend(already_entered);
    }
  }

  fn hit_widget(&self, tree: &WidgetTree) -> Option<WidgetId> {
    fn down_coordinate(id: WidgetId, pos: Point, tree: &WidgetTree) -> Option<(WidgetId, Point)> {
      let WidgetTree { arena, store, wnd_ctx, .. } = tree;

      let r = id.assert_get(arena);
      let ctx = HitTestCtx { id, arena, store, wnd_ctx };
      let hit_test = r.hit_test(&ctx, pos);

      if hit_test.hit {
        Some((id, store.map_from_parent(id, pos, arena)))
      } else if hit_test.can_hit_child {
        let pos = store.map_from_parent(id, pos, arena);
        id.reverse_children(arena)
          .find_map(|c| down_coordinate(c, pos, tree))
      } else {
        None
      }
    }

    let mut current = down_coordinate(tree.root(), self.info.borrow().cursor_pos(), tree);
    let mut hit = current;
    while let Some((id, pos)) = current {
      hit = current;
      current = id
        .reverse_children(&tree.arena)
        .find_map(|c| down_coordinate(c, pos, tree));
    }
    hit.map(|(w, _)| w)
  }

  fn pointer_event_for_hit_widget(&mut self, tree: &WidgetTree) -> Option<PointerEvent> {
    self
      .hit_widget(tree)
      .map(|target| PointerEvent::from_mouse(target, tree, self.info.clone()))
  }
}

impl Dispatcher {
  pub fn next_focus_widget(&mut self, tree: &WidgetTree) {
    self.focus_mgr.borrow_mut().next_focus(&tree.arena);
  }

  pub fn prev_focus_widget(&mut self, tree: &WidgetTree) {
    self.focus_mgr.borrow_mut().prev_focus(&tree.arena);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self, tree: &mut WidgetTree) -> Option<WidgetId> {
    self.change_focusing_to(None, tree)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focus_mgr.borrow_mut().focusing }

  pub fn refresh_focus(&mut self, tree: &WidgetTree) {
    let focusing = self.focus_mgr.borrow().focusing.filter(|node_id| {
      self
        .focus_mgr
        .borrow()
        .ignore_scope_id(*node_id, &tree.arena)
        .is_none()
    });
    if self.focus_widgets.get(0) != focusing.as_ref() {
      self.change_focusing_to(focusing, tree);
    }
  }

  pub fn focus(&mut self, wid: WidgetId, tree: &WidgetTree) {
    self.change_focusing_to(Some(wid), tree);
  }

  fn change_focusing_to(&mut self, node: Option<WidgetId>, tree: &WidgetTree) -> Option<WidgetId> {
    let Self { focus_mgr, info, .. } = self;
    let old_widgets = &self.focus_widgets;
    let new_widgets = node.map_or(vec![], |wid| wid.ancestors(&tree.arena).collect::<Vec<_>>());

    let old = old_widgets
      .get(0)
      .filter(|wid| !(*wid).is_dropped(&tree.arena))
      .copied();

    // dispatch blur event
    if let Some(wid) = old {
      let mut focus_event = FocusEvent::new(wid, tree, info.clone());
      wid
        .assert_get(&tree.arena)
        .query_on_first_type(QueryOrder::InnerFirst, |blur: &BlurListener| {
          blur.dispatch(&mut focus_event)
        })
    };

    let common_ancestors = common_ancestors(&new_widgets, old_widgets);
    // bubble focus out
    if let Some(wid) = old_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(&tree.arena))
    {
      let mut focus_event = FocusEvent::new(*wid, tree, info.clone());
      tree.bubble_event_with(&mut focus_event, |focus_out: &FocusOutListener, event| {
        if common_ancestors.contains(&event.current_target()) {
          event.stop_bubbling();
        } else {
          focus_out.dispatch(event);
        }
      });
    };

    if let Some(wid) = node {
      let mut focus_event = FocusEvent::new(wid, tree, info.clone());

      wid
        .assert_get(&tree.arena)
        .query_on_first_type(QueryOrder::InnerFirst, |focus: &FocusListener| {
          focus.dispatch(&mut focus_event)
        });

      let mut focus_event = FocusEvent::new(wid, tree, info.clone());

      // bubble focus in
      tree.bubble_event_with(&mut focus_event, |focus_in: &FocusInListener, event| {
        if common_ancestors.contains(&event.current_target()) {
          event.stop_bubbling();
        } else {
          focus_in.dispatch(event);
        }
      });
    }

    self.focus_widgets = new_widgets;
    focus_mgr.borrow_mut().focusing = node;
    old
  }
}
