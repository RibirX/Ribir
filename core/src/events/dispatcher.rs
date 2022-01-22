use crate::{context::Context, prelude::*};
mod focus_mgr;
pub(crate) use focus_mgr::FocusManager;
mod pointer;
pub(crate) use pointer::PointerDispatcher;
mod util;
use rxrust::prelude::*;
use std::rc::Rc;
pub(crate) use util::*;
pub use window::RawWindow;
use winit::event::{ElementState, WindowEvent};

#[derive(Default)]
pub(crate) struct Dispatcher {
  pub(crate) pointer: PointerDispatcher,
  pub(crate) focus_mgr: FocusManager,
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, ctx: &mut Context) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => ctx.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => self
        .pointer
        .cursor_move_to(Point::new(position.x as f32, position.y as f32), ctx),
      WindowEvent::CursorLeft { .. } => self.pointer.on_cursor_left(ctx),
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        self
          .pointer
          .dispatch_mouse_input(device_id, state, button, ctx, &mut self.focus_mgr);
      }
      WindowEvent::KeyboardInput { input, .. } => {
        self.dispatch_keyboard_input(input, ctx);
      }
      WindowEvent::ReceivedCharacter(c) => {
        self.dispatch_received_char(c, ctx);
      }
      WindowEvent::MouseWheel { delta, .. } => self.pointer.dispatch_wheel(delta, ctx),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  pub fn dispatch_keyboard_input(&mut self, input: winit::event::KeyboardInput, ctx: &mut Context) {
    if let Some(key) = input.virtual_keycode {
      let prevented = if let Some(focus) = self.focus_mgr.focusing() {
        let event = KeyboardEvent {
          key,
          scan_code: input.scancode,
          common: EventCommon::new(focus, ctx),
        };
        let event_type = match input.state {
          ElementState::Pressed => KeyboardEventType::KeyDown,
          ElementState::Released => KeyboardEventType::KeyUp,
        };
        let event = bubble_dispatch(
          |keyboard: &KeyboardAttr| KeyBoardObserver::new(keyboard, event_type),
          event,
          |_| {},
        );
        event.common.prevent_default.get()
      } else {
        false
      };
      if !prevented {
        self.shortcut_process(key, ctx);
      }
    }
  }

  pub fn dispatch_received_char(&mut self, c: char, ctx: &mut Context) {
    if let Some(focus) = self.focus_mgr.focusing() {
      let event = CharEvent {
        char: c,
        common: EventCommon::new(focus, ctx),
      };
      util::bubble_dispatch(|attr: &CharAttr| attr.event_observable(), event, |_| {});
    }
  }

  pub fn shortcut_process(&mut self, key: VirtualKeyCode, ctx: &mut Context) {
    if key == VirtualKeyCode::Tab {
      if ctx.modifiers.contains(ModifiersState::SHIFT) {
        self.focus_mgr.prev_focus_widget(ctx);
      } else {
        self.focus_mgr.next_focus_widget(ctx);
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
