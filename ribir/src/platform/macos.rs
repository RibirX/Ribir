use crate::prelude::{App, AppEvent, HotkeyEvent};
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
use winit::event::{ModifiersState, VirtualKeyCode};

use std::{ptr::NonNull, sync::Once, time::Duration};

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
      &self,
      handler: &AnyObject,
      and_selector: Sel,
      for_event_class: u32,
      and_event_id: u32,
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
        if cstr != std::ptr::null() {
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
            println!("interval complete");
            add_global_monitor_for_events_matching_mask_handler();
          })
          .subscribe(|_| {
            println!("interval tick");
          });
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
    modifiers_state.insert(ModifiersState::LOGO);
  }
  if (modifiers & NSEventModifierFlagShift) == NSEventModifierFlagShift {
    modifiers_state.insert(ModifiersState::SHIFT);
  }
  if (modifiers & NSEventModifierFlagControl) == NSEventModifierFlagControl {
    modifiers_state.insert(ModifiersState::CTRL);
  }
  if (modifiers & NSEventModifierFlagOption) == NSEventModifierFlagOption {
    modifiers_state.insert(ModifiersState::ALT);
  }

  if !modifiers_state.is_empty() {
    Some(modifiers_state)
  } else {
    None
  }
}

fn scancode_to_key(key_code: u32) -> Option<VirtualKeyCode> {
  match key_code {
    0x00 => Some(VirtualKeyCode::A),
    0x01 => Some(VirtualKeyCode::S),
    0x02 => Some(VirtualKeyCode::D),
    0x03 => Some(VirtualKeyCode::F),
    0x04 => Some(VirtualKeyCode::H),
    0x05 => Some(VirtualKeyCode::G),
    0x06 => Some(VirtualKeyCode::Z),
    0x07 => Some(VirtualKeyCode::X),
    0x08 => Some(VirtualKeyCode::C),
    0x09 => Some(VirtualKeyCode::V),
    0x0B => Some(VirtualKeyCode::B),
    0x0C => Some(VirtualKeyCode::Q),
    0x0D => Some(VirtualKeyCode::W),
    0x0E => Some(VirtualKeyCode::E),
    0x0F => Some(VirtualKeyCode::R),
    0x10 => Some(VirtualKeyCode::Y),
    0x11 => Some(VirtualKeyCode::T),
    0x12 => Some(VirtualKeyCode::Key1),
    0x13 => Some(VirtualKeyCode::Key2),
    0x14 => Some(VirtualKeyCode::Key3),
    0x15 => Some(VirtualKeyCode::Key4),
    0x16 => Some(VirtualKeyCode::Key6),
    0x17 => Some(VirtualKeyCode::Key5),
    0x18 => Some(VirtualKeyCode::Equals),
    0x19 => Some(VirtualKeyCode::Key9),
    0x1A => Some(VirtualKeyCode::Key7),
    0x1B => Some(VirtualKeyCode::Minus),
    0x1C => Some(VirtualKeyCode::Key8),
    0x1D => Some(VirtualKeyCode::Key0),
    0x1E => Some(VirtualKeyCode::RBracket),
    0x1F => Some(VirtualKeyCode::O),
    0x20 => Some(VirtualKeyCode::U),
    0x21 => Some(VirtualKeyCode::LBracket),
    0x22 => Some(VirtualKeyCode::I),
    0x23 => Some(VirtualKeyCode::P),
    0x24 => Some(VirtualKeyCode::Return),
    0x25 => Some(VirtualKeyCode::L),
    0x26 => Some(VirtualKeyCode::J),
    0x27 => Some(VirtualKeyCode::Apostrophe),
    0x28 => Some(VirtualKeyCode::K),
    0x29 => Some(VirtualKeyCode::Semicolon),
    0x2A => Some(VirtualKeyCode::Backslash),
    0x2B => Some(VirtualKeyCode::Comma),
    0x2C => Some(VirtualKeyCode::Slash),
    0x2D => Some(VirtualKeyCode::N),
    0x2E => Some(VirtualKeyCode::M),
    0x2F => Some(VirtualKeyCode::Period),
    0x30 => Some(VirtualKeyCode::Tab),
    0x31 => Some(VirtualKeyCode::Space),
    0x32 => Some(VirtualKeyCode::Grave),
    0x33 => Some(VirtualKeyCode::Back),
    0x35 => Some(VirtualKeyCode::Escape),
    0x40 => Some(VirtualKeyCode::F17),
    0x41 => Some(VirtualKeyCode::NumpadDecimal),
    0x43 => Some(VirtualKeyCode::NumpadMultiply),
    0x45 => Some(VirtualKeyCode::NumpadAdd),
    0x47 => Some(VirtualKeyCode::Numlock),
    0x48 => Some(VirtualKeyCode::VolumeUp),
    0x49 => Some(VirtualKeyCode::VolumeDown),
    0x4A => Some(VirtualKeyCode::Mute),
    0x4B => Some(VirtualKeyCode::NumpadDivide),
    0x4C => Some(VirtualKeyCode::NumpadEnter),
    0x4E => Some(VirtualKeyCode::NumpadSubtract),
    0x4F => Some(VirtualKeyCode::F18),
    0x50 => Some(VirtualKeyCode::F19),
    0x51 => Some(VirtualKeyCode::NumpadEquals),
    0x52 => Some(VirtualKeyCode::Numpad0),
    0x53 => Some(VirtualKeyCode::Numpad1),
    0x54 => Some(VirtualKeyCode::Numpad2),
    0x55 => Some(VirtualKeyCode::Numpad3),
    0x56 => Some(VirtualKeyCode::Numpad4),
    0x57 => Some(VirtualKeyCode::Numpad5),
    0x58 => Some(VirtualKeyCode::Numpad6),
    0x59 => Some(VirtualKeyCode::Numpad7),
    0x5A => Some(VirtualKeyCode::F20),
    0x5B => Some(VirtualKeyCode::Numpad8),
    0x5C => Some(VirtualKeyCode::Numpad9),
    0x60 => Some(VirtualKeyCode::F5),
    0x61 => Some(VirtualKeyCode::F6),
    0x62 => Some(VirtualKeyCode::F7),
    0x63 => Some(VirtualKeyCode::F3),
    0x64 => Some(VirtualKeyCode::F8),
    0x65 => Some(VirtualKeyCode::F9),
    0x67 => Some(VirtualKeyCode::F11),
    0x69 => Some(VirtualKeyCode::F13),
    0x6A => Some(VirtualKeyCode::F16),
    0x6B => Some(VirtualKeyCode::F14),
    0x6D => Some(VirtualKeyCode::F10),
    0x6F => Some(VirtualKeyCode::F12),
    0x71 => Some(VirtualKeyCode::F15),
    0x72 => Some(VirtualKeyCode::Insert),
    0x73 => Some(VirtualKeyCode::Home),
    0x74 => Some(VirtualKeyCode::PageUp),
    0x75 => Some(VirtualKeyCode::Delete),
    0x76 => Some(VirtualKeyCode::F4),
    0x77 => Some(VirtualKeyCode::End),
    0x78 => Some(VirtualKeyCode::F2),
    0x79 => Some(VirtualKeyCode::PageDown),
    0x7A => Some(VirtualKeyCode::F1),
    0x7B => Some(VirtualKeyCode::Left),
    0x7C => Some(VirtualKeyCode::Right),
    0x7D => Some(VirtualKeyCode::Down),
    0x7E => Some(VirtualKeyCode::Up),
    0x39 => Some(VirtualKeyCode::Capital),
    0x46 => Some(VirtualKeyCode::Snapshot),
    _ => None,
  }
}
