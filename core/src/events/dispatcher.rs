use crate::{context::Context, prelude::*};
mod focus_mgr;
pub(crate) use focus_mgr::FocusManager;
mod pointer;
pub(crate) use pointer::PointerDispatcher;

use winit::event::{ElementState, WindowEvent};

#[derive(Default)]
pub(crate) struct Dispatcher {
  pub(crate) pointer: PointerDispatcher,
  pub(crate) focus_mgr: FocusManager,
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, ctx: &mut Context, wnd_factor: f64) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => ctx.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        let pos = position.to_logical::<f32>(wnd_factor);
        self.pointer.cursor_move_to(Point::new(pos.x, pos.y), ctx)
      }
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
        let event_type = match input.state {
          ElementState::Pressed => KeyboardEventType::KeyDown,
          ElementState::Released => KeyboardEventType::KeyUp,
        };

        let event = ctx.bubble_event(
          focus,
          |ctx, id| KeyboardEvent {
            key,
            scan_code: input.scancode,
            common: EventCommon::new(id, ctx),
          },
          |keyboard: &KeyboardAttr, event| keyboard.dispatch_event(event_type, event),
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
      ctx.bubble_event(
        focus,
        |ctx, id| CharEvent {
          char: c,
          common: EventCommon::new(id, ctx),
        },
        |attr: &CharAttr, event| attr.dispatch_event(event),
      );
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
