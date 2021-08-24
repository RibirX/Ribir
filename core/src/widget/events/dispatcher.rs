use crate::{prelude::*, render::render_tree::RenderTree, widget::widget_tree::WidgetTree};
mod focus_mgr;
pub(crate) use focus_mgr::FocusManager;
mod pointer;
pub(crate) use pointer::PointerDispatcher;
mod common;
pub(crate) use common::CommonDispatcher;
use rxrust::prelude::*;
use std::{cell::RefCell, ptr::NonNull, rc::Rc};
pub use window::RawWindow;
use winit::event::{ElementState, WindowEvent};

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
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        self.pointer.dispatch_mouse_input(
          device_id,
          state,
          button,
          &self.common,
          &mut self.focus_mgr,
        );
      }
      WindowEvent::KeyboardInput { input, .. } => {
        self.dispatch_keyboard_input(input);
      }
      WindowEvent::ReceivedCharacter(c) => {
        self.dispatch_received_char(c);
      }
      WindowEvent::MouseWheel { delta, .. } => self.pointer.dispatch_wheel(delta, &self.common),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  pub fn dispatch_keyboard_input(&mut self, input: winit::event::KeyboardInput) {
    if let Some(key) = input.virtual_keycode {
      let prevented = if let Some(focus) = self.focus_mgr.focusing() {
        let event = KeyboardEvent {
          key,
          scan_code: input.scancode,
          common: EventCommon::new(
            self.common.modifiers,
            focus,
            self.common.window.clone(),
            self.common.widget_tree,
            self.common.render_tree,
          ),
        };
        let event_type = match input.state {
          ElementState::Pressed => KeyboardEventType::KeyDown,
          ElementState::Released => KeyboardEventType::KeyUp,
        };
        let event = self.common.bubble_dispatch(
          focus,
          |keyboard: &KeyboardAttr| KeyBoardObserver::new(keyboard, event_type),
          event,
          |_| {},
        );
        event.common.prevent_default.get()
      } else {
        false
      };
      if !prevented {
        self.shortcut_process(key);
      }
    }
  }

  pub fn dispatch_received_char(&mut self, c: char) {
    if let Some(focus) = self.focus_mgr.focusing() {
      let event = CharEvent {
        char: c,
        common: EventCommon::new(
          self.common.modifiers,
          focus,
          self.common.window.clone(),
          self.common.widget_tree,
          self.common.render_tree,
        ),
      };
      self.common.bubble_dispatch(
        focus,
        |attr: &CharAttr| attr.event_observable(),
        event,
        |_| {},
      );
    }
  }

  pub fn shortcut_process(&mut self, key: VirtualKeyCode) {
    if key == VirtualKeyCode::Tab {
      if self.common.modifiers.contains(ModifiersState::SHIFT) {
        self.focus_mgr.prev_focus_widget(&self.common);
      } else {
        self.focus_mgr.next_focus_widget(&self.common);
      }
    }
  }
}

struct KeyBoardObserver {
  event_type: KeyboardEventType,
  subject: LocalSubject<'static, (KeyboardEventType, Rc<KeyboardEvent>), ()>,
}

impl KeyBoardObserver {
  fn new(attr: &KeyboardAttr, event_type: KeyboardEventType) -> Self {
    Self {
      event_type,
      subject: attr.event_observable(),
    }
  }
}

impl Observer for KeyBoardObserver {
  type Item = Rc<KeyboardEvent>;
  type Err = ();

  fn next(&mut self, value: Self::Item) { self.subject.next((self.event_type, value)) }

  fn error(&mut self, err: Self::Err) { self.subject.error(err); }

  fn complete(&mut self) { self.subject.complete() }
}
