use crate::prelude::{App, AppEvent};
use std::sync::Once;

use objc2::{
  declare_class, extern_class, extern_methods,
  foundation::NSObject,
  msg_send, msg_send_id,
  rc::{Id, Owned, Shared},
  runtime::{Object, Sel},
  sel, ClassType,
};

extern_class!(
  #[derive(Debug, PartialEq, Eq, Hash)]
  pub(crate) struct NSAppleEventManager;

  unsafe impl ClassType for NSAppleEventManager {
    type Super = NSObject;
  }
);

extern_methods!(
  unsafe impl NSAppleEventManager {
    fn shared() -> Id<Self, Shared> {
      unsafe {
        let manager: Option<_> = msg_send_id![Self::class(), sharedAppleEventManager];
        manager.unwrap_unchecked()
      }
    }

    #[sel(setEventHandler:andSelector:forEventClass:andEventID:)]
    fn set_event_handler(
      &self,
      handler: &Object,
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
  }

  unsafe impl AppEventsHandler {
    #[sel(openUrl:withReplyEvent:)]
    fn open_url(&self, event: &Object, _with_reply_event: &Object) {
      let url = unsafe {
        let class: u32 = msg_send![event, eventClass];
        let id: u32 = msg_send![event, eventID];
        if class != kInternetEventClass || id != kAEGetURL {
          return;
        }
        let event: *mut Object = msg_send![event, paramDescriptorForKeyword: keyDirectObject];
        let nsstring: *mut Object = msg_send![event, stringValue];
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
  static mut APP_EVENTS_HANDLERS: Option<Id<AppEventsHandler, Owned>> = None;
  static INIT: Once = Once::new();
  unsafe {
    INIT.call_once(|| {
      let handler: Id<AppEventsHandler, Owned> = msg_send_id![AppEventsHandler::class(), new];
      let manager = NSAppleEventManager::shared();
      manager.set_event_handler(
        &handler,
        sel!(openUrl:withReplyEvent:),
        kInternetEventClass,
        kAEGetURL,
      );
      APP_EVENTS_HANDLERS = Some(handler);
    });
  }
}
