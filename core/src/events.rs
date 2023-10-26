use self::dispatcher::DispatchInfo;
use crate::{
  context::{define_widget_context, WidgetCtx, WidgetCtxImpl},
  prelude::AppCtx,
  widget_tree::WidgetId,
  window::{Window, WindowId},
};
use std::rc::Rc;

pub(crate) mod dispatcher;
mod pointers;
pub use pointers::*;
use ribir_geom::Point;
mod focus;
pub use focus::*;
mod keyboard;
pub use keyboard::*;
mod character;
pub use character::*;
mod wheel;
pub use wheel::*;
pub use winit::keyboard::{Key as VirtualKey, KeyCode, ModifiersState, NamedKey, PhysicalKey};
pub(crate) mod focus_mgr;
mod listener_impl_helper;

define_widget_context!(
  CommonEvent,
  target: WidgetId,
  propagation: bool,
  prevent_default: bool
);

impl CommonEvent {
  /// The target property of the Event interface is a reference to the object
  /// onto which the event was dispatched. It is different from
  /// Event::current_target when the event handler is called during the bubbling
  /// phase of the event.
  #[inline]
  pub fn target(&self) -> WidgetId { self.target }
  /// A reference to the currently registered target for the event. This is the
  /// object to which the event is currently slated to be sent. It's possible
  /// this has been changed along the way through retargeting.
  #[inline]
  pub fn current_target(&self) -> WidgetId { self.id }
  /// Prevent event bubbling to parent.
  #[inline]
  pub fn stop_propagation(&mut self) { self.propagation = false }
  /// Whether the event is bubbling or not.
  #[inline]
  pub fn is_propagation(&self) -> bool { self.propagation }
  /// Tells the user agent that if the event does not get explicitly handled,
  /// its default action should not be taken as it normally would be.
  #[inline]
  pub fn prevent_default(&mut self) { self.prevent_default = true; }

  /// Whether the event is prevented the default action or not.
  #[inline]
  pub(crate) fn is_prevent_default(&self) -> bool { self.prevent_default }

  /// Represents the current state of the keyboard modifiers
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.pick_info(DispatchInfo::modifiers) }

  /// Returns `true` if the shift key is pressed.
  pub fn with_shift_key(&self) -> bool { self.modifiers().shift_key() }

  /// Returns `true` if the alt key is pressed.
  pub fn with_alt_key(&self) -> bool { self.modifiers().alt_key() }

  /// Returns `true` if the ctrl key is pressed.
  pub fn with_ctrl_key(&self) -> bool { self.modifiers().control_key() }
  /// Returns `true` if the logo key is pressed.
  pub fn with_logo_key(&self) -> bool { self.modifiers().super_key() }

  /// Returns true if the main modifier key in the
  /// current platform is pressed. Specifically:
  /// - the `logo` or command key (⌘) on macOS
  /// - the `control` key on other platforms
  pub fn with_command_key(&self) -> bool {
    #[cfg(target_os = "macos")]
    return self.with_logo_key();

    #[cfg(not(target_os = "macos"))]
    return self.with_ctrl_key();
  }

  /// The X, Y coordinate of the mouse pointer in global (window) coordinates.
  #[inline]
  pub fn global_pos(&self) -> Point { self.pick_info(DispatchInfo::global_pos) }

  /// The X, Y coordinate of the pointer in current target widget.
  #[inline]
  pub fn position(&self) -> Point { self.map_from_global(self.global_pos()) }

  /// The buttons being depressed (if any) in current state.
  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.pick_info(DispatchInfo::mouse_buttons) }

  /// The button number that was pressed (if applicable) when the mouse event
  /// was fired.
  #[inline]
  pub fn button_num(&self) -> u32 { self.mouse_buttons().bits().count_ones() }
}

pub trait EventListener {
  type Event;
  fn dispatch(&self, event: &mut Self::Event);
}

impl std::fmt::Debug for CommonEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CommonEvent")
      .field("target", &self.id)
      .field("current_target", &self.id)
      .field("is_propagation", &self.propagation)
      .finish()
  }
}

impl CommonEvent {
  /// Create a new common event.
  ///
  /// Although the `dispatcher` is contained in the `wnd`, we still need to pass
  /// it because in most case the event create in a environment that the
  /// `Dispatcher` already borrowed.
  pub(crate) fn new(target: WidgetId, wnd_id: WindowId) -> Self {
    Self {
      target,
      wnd_id,
      id: target,
      propagation: true,
      prevent_default: false,
    }
  }

  pub(crate) fn set_current_target(&mut self, id: WidgetId) { self.id = id; }

  fn pick_info<R>(&self, f: impl FnOnce(&DispatchInfo) -> R) -> R {
    f(&self.current_wnd().dispatcher.borrow().info)
  }
}
