use std::{ptr::NonNull, sync::Once, time::Duration};

use icrate::{
  block2::ConcreteBlock,
  objc2::{
    rc::Id,
    runtime::{AnyObject, Sel},
    *,
  },
  AppKit::{
    NSEvent, NSEventMaskKeyDown, NSEventModifierFlagCommand, NSEventModifierFlagControl,
    NSEventModifierFlagOption, NSEventModifierFlagShift,
  },
  Foundation::NSObject,
};
use ribir_core::prelude::AppCtx;
use rxrust::prelude::{interval, ObservableExt, ObservableItem};
use winit::keyboard::{KeyCode, ModifiersState};

use crate::prelude::{App, AppEvent, HotkeyEvent};

extern_class!(
  #[derive(Debug, PartialEq, Eq, Hash)]
  pub(crate) struct NSAppleEventManager;

  unsafe impl ClassType for NSAppleEventManager {
    type Super = NSObject;
    type Mutability = mutability::Mutable;
    const NAME: &'static str = "NSAppleEventManager";
  }
);

extern_methods!(
  unsafe impl NSAppleEventManager {
    fn shared() -> Id<Self> {
      unsafe {
        let manager: Option<_> = msg_send_id![Self::class(), sharedAppleEventManager];
        manager.unwrap_unchecked()
      }
    }

    #[method(setEventHandler:andSelector:forEventClass:andEventID:)]
    fn set_event_handler(
      &self, handler: &AnyObject, and_selector: Sel, for_event_class: u32, and_event_id: u32,
    );
  }
);

declare_class!(
  struct AppEventsHandler {}

  unsafe impl ClassType for AppEventsHandler {
    type Super = NSObject;
    type Mutability = mutability::Mutable;
    const NAME: &'static str = "AppEventsHandler";
  }

  unsafe impl AppEventsHandler {
    #[method(openUrl:withReplyEvent:)]
    fn open_url(&self, event: &NSEvent, _with_reply_event: &AnyObject) {
      let url = unsafe {
        let class: u32 = msg_send![event, eventClass];
        let id: u32 = msg_send![event, eventID];

        if class != kInternetEventClass || id != kAEGetURL {
          return;
        }
        let event: *mut AnyObject = msg_send![event, paramDescriptorForKeyword: keyDirectObject];
        let nsstring: *mut AnyObject = msg_send![event, stringValue];
        let cstr: *const i8 = msg_send![nsstring, UTF8String];
        if cstr.is_null() {
          std::ffi::CStr::from_ptr(cstr)
        } else {
          return;
        }
      };
      let url = url.to_string_lossy().into_owned();
      App::event_sender().send(AppEvent::OpenUrl(url));
    }
  }
);

/// Apple kInternetEventClass constant
#[allow(non_upper_case_globals)]
pub const kInternetEventClass: u32 = 0x4755524c;
/// Apple kAEGetURL constant
#[allow(non_upper_case_globals)]
pub const kAEGetURL: u32 = 0x4755524c;
/// Apple keyDirectObject constant
#[allow(non_upper_case_globals)]
pub const keyDirectObject: u32 = 0x2d2d2d2d;

pub fn register_platform_app_events_handlers() {
  static mut APP_EVENTS_HANDLERS: Option<Id<AppEventsHandler>> = None;
  static INIT: Once = Once::new();
  unsafe {
    INIT.call_once(|| {
      let handler: Id<AppEventsHandler> = msg_send_id![AppEventsHandler::class(), new];
      let manager = NSAppleEventManager::shared();
      manager.set_event_handler(
        &handler,
        sel!(openUrl:withReplyEvent:),
        kInternetEventClass,
        kAEGetURL,
      );
      APP_EVENTS_HANDLERS = Some(handler);

      if query_accessibility_permissions() {
        add_global_monitor_for_events_matching_mask_handler();
      } else {
        let _ = interval(Duration::from_secs(5), AppCtx::scheduler())
          .take_while(|_| !query_accessibility_permissions())
          .on_complete(|| {
            add_global_monitor_for_events_matching_mask_handler();
          })
          .subscribe(|_| {});
      }
    });
  }
}

fn add_global_monitor_for_events_matching_mask_handler() -> Option<Id<AnyObject>> {
  unsafe {
    NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
      NSEventMaskKeyDown,
      &ConcreteBlock::new(|e: NonNull<NSEvent>| {
        let key_code = scancode_to_key(e.as_ref().keyCode() as u32);
        let modifiers = modifier_flag(e.as_ref().modifierFlags());
        match (key_code, modifiers) {
          (None, None) => {}
          _ => {
            App::event_sender().send(AppEvent::Hotkey(HotkeyEvent { key_code, modifiers }));
          }
        }
      }),
    )
  }
}

#[cfg(target_os = "macos")]
fn query_accessibility_permissions() -> bool {
  macos_accessibility_client::accessibility::application_is_trusted()
}

fn modifier_flag(modifiers: usize) -> Option<ModifiersState> {
  let mut modifiers_state = ModifiersState::empty();

  if (modifiers & NSEventModifierFlagCommand) == NSEventModifierFlagCommand {
    modifiers_state.insert(ModifiersState::SUPER);
  }
  if (modifiers & NSEventModifierFlagShift) == NSEventModifierFlagShift {
    modifiers_state.insert(ModifiersState::SHIFT);
  }
  if (modifiers & NSEventModifierFlagControl) == NSEventModifierFlagControl {
    modifiers_state.insert(ModifiersState::CONTROL);
  }
  if (modifiers & NSEventModifierFlagOption) == NSEventModifierFlagOption {
    modifiers_state.insert(ModifiersState::ALT);
  }

  if !modifiers_state.is_empty() { Some(modifiers_state) } else { None }
}

fn scancode_to_key(key_code: u32) -> Option<KeyCode> {
  match key_code {
    0x00 => Some(KeyCode::KeyA),
    0x01 => Some(KeyCode::KeyS),
    0x02 => Some(KeyCode::KeyD),
    0x03 => Some(KeyCode::KeyF),
    0x04 => Some(KeyCode::KeyH),
    0x05 => Some(KeyCode::KeyG),
    0x06 => Some(KeyCode::KeyZ),
    0x07 => Some(KeyCode::KeyX),
    0x08 => Some(KeyCode::KeyC),
    0x09 => Some(KeyCode::KeyV),
    0x0B => Some(KeyCode::KeyB),
    0x0C => Some(KeyCode::KeyQ),
    0x0D => Some(KeyCode::KeyW),
    0x0E => Some(KeyCode::KeyE),
    0x0F => Some(KeyCode::KeyR),
    0x10 => Some(KeyCode::KeyY),
    0x11 => Some(KeyCode::KeyT),
    0x12 => Some(KeyCode::Digit1),
    0x13 => Some(KeyCode::Digit2),
    0x14 => Some(KeyCode::Digit3),
    0x15 => Some(KeyCode::Digit4),
    0x16 => Some(KeyCode::Digit6),
    0x17 => Some(KeyCode::Digit5),
    0x18 => Some(KeyCode::Equal),
    0x19 => Some(KeyCode::Digit9),
    0x1A => Some(KeyCode::Digit7),
    0x1B => Some(KeyCode::Minus),
    0x1C => Some(KeyCode::Digit8),
    0x1D => Some(KeyCode::Digit0),
    0x1E => Some(KeyCode::BracketRight),
    0x1F => Some(KeyCode::KeyO),
    0x20 => Some(KeyCode::KeyU),
    0x21 => Some(KeyCode::BracketLeft),
    0x22 => Some(KeyCode::KeyI),
    0x23 => Some(KeyCode::KeyP),
    0x24 => Some(KeyCode::Enter),
    0x25 => Some(KeyCode::KeyL),
    0x26 => Some(KeyCode::KeyJ),
    0x27 => Some(KeyCode::Quote),
    0x28 => Some(KeyCode::KeyK),
    0x29 => Some(KeyCode::Semicolon),
    0x2A => Some(KeyCode::Backslash),
    0x2B => Some(KeyCode::Comma),
    0x2C => Some(KeyCode::Slash),
    0x2D => Some(KeyCode::KeyN),
    0x2E => Some(KeyCode::KeyM),
    0x2F => Some(KeyCode::Period),
    0x30 => Some(KeyCode::Tab),
    0x31 => Some(KeyCode::Space),
    0x32 => Some(KeyCode::Backquote),
    0x33 => Some(KeyCode::Backspace),
    0x35 => Some(KeyCode::Escape),
    0x40 => Some(KeyCode::F17),
    0x41 => Some(KeyCode::NumpadDecimal),
    0x43 => Some(KeyCode::NumpadMultiply),
    0x45 => Some(KeyCode::NumpadAdd),
    0x47 => Some(KeyCode::NumLock),
    0x48 => Some(KeyCode::AudioVolumeUp),
    0x49 => Some(KeyCode::AudioVolumeDown),
    0x4A => Some(KeyCode::AudioVolumeUp),
    0x4B => Some(KeyCode::NumpadDivide),
    0x4C => Some(KeyCode::NumpadEnter),
    0x4E => Some(KeyCode::NumpadSubtract),
    0x4F => Some(KeyCode::F18),
    0x50 => Some(KeyCode::F19),
    0x51 => Some(KeyCode::NumpadEqual),
    0x52 => Some(KeyCode::Numpad0),
    0x53 => Some(KeyCode::Numpad1),
    0x54 => Some(KeyCode::Numpad2),
    0x55 => Some(KeyCode::Numpad3),
    0x56 => Some(KeyCode::Numpad4),
    0x57 => Some(KeyCode::Numpad5),
    0x58 => Some(KeyCode::Numpad6),
    0x59 => Some(KeyCode::Numpad7),
    0x5A => Some(KeyCode::F20),
    0x5B => Some(KeyCode::Numpad8),
    0x5C => Some(KeyCode::Numpad9),
    0x60 => Some(KeyCode::F5),
    0x61 => Some(KeyCode::F6),
    0x62 => Some(KeyCode::F7),
    0x63 => Some(KeyCode::F3),
    0x64 => Some(KeyCode::F8),
    0x65 => Some(KeyCode::F9),
    0x67 => Some(KeyCode::F11),
    0x69 => Some(KeyCode::F13),
    0x6A => Some(KeyCode::F16),
    0x6B => Some(KeyCode::F14),
    0x6D => Some(KeyCode::F10),
    0x6F => Some(KeyCode::F12),
    0x71 => Some(KeyCode::F15),
    0x72 => Some(KeyCode::Insert),
    0x73 => Some(KeyCode::Home),
    0x74 => Some(KeyCode::PageUp),
    0x75 => Some(KeyCode::Delete),
    0x76 => Some(KeyCode::F4),
    0x77 => Some(KeyCode::End),
    0x78 => Some(KeyCode::F2),
    0x79 => Some(KeyCode::PageDown),
    0x7A => Some(KeyCode::F1),
    0x7B => Some(KeyCode::ArrowLeft),
    0x7C => Some(KeyCode::ArrowRight),
    0x7D => Some(KeyCode::ArrowDown),
    0x7E => Some(KeyCode::ArrowUp),
    0x39 => Some(KeyCode::CapsLock),
    0x46 => Some(KeyCode::PrintScreen),
    _ => None,
  }
}
